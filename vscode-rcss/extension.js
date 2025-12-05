const vscode = require("vscode");
const { exec } = require("child_process");
const path = require("path");

let outputChannel;
const activeBuilds = new Map();

function activate(context) {
  outputChannel = vscode.window.createOutputChannel("RCSS");

  const buildCommand = vscode.commands.registerCommand("rcss.buildFile", () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      vscode.window.showErrorMessage("Open an RCSS file to build.");
      return;
    }

    const { document } = editor;
    if (!isRcssDocument(document)) {
      vscode.window.showErrorMessage("Current file is not an .rcss file.");
      return;
    }

    const config = vscode.workspace.getConfiguration("rcss");
    triggerBuild(document.fileName, config);
  });

  const onSave = vscode.workspace.onDidSaveTextDocument((doc) => {
    if (!isRcssDocument(doc)) {
      return;
    }

    const config = vscode.workspace.getConfiguration("rcss");
    if (!config.get("buildOnSave", true)) {
      return;
    }

    triggerBuild(doc.fileName, config);
  });

  context.subscriptions.push(buildCommand, onSave, outputChannel);
}

function isRcssDocument(doc) {
  return doc.languageId === "rcss" || doc.fileName.toLowerCase().endsWith(".rcss");
}

function triggerBuild(filePath, config) {
  if (activeBuilds.has(filePath)) {
    return activeBuilds.get(filePath);
  }

  const commandTemplate =
    config.get("buildCommand") || "cargo run -p rcss-cli -- build ${file}";
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(
    vscode.Uri.file(filePath),
  );
  const configuredCwd = (config.get("buildCwd") || "").trim();
  const cwd =
    configuredCwd ||
    workspaceFolder?.uri.fsPath ||
    path.dirname(filePath);

  const command = commandTemplate.replace(/\${file}/g, shellQuote(filePath));

  outputChannel.appendLine(`$ ${command}`);

  const buildPromise = new Promise((resolve) => {
    const child = exec(
      command,
      { cwd, shell: true },
      (error, stdout, stderr) => {
        if (stdout) {
          outputChannel.append(stdout);
        }
        if (stderr) {
          outputChannel.append(stderr);
        }

        if (error) {
          vscode.window.showErrorMessage(
            `RCSS build failed for ${path.basename(filePath)}. See RCSS output for details.`,
          );
        } else {
          vscode.window.setStatusBarMessage(
            `RCSS built ${path.basename(filePath)}`,
            2000,
          );
        }
        resolve();
      },
    );

    child.on("error", (err) => {
      outputChannel.appendLine(String(err));
      vscode.window.showErrorMessage(
        `RCSS build could not start. Check the RCSS output channel.`,
      );
      resolve();
    });
  }).finally(() => activeBuilds.delete(filePath));

  activeBuilds.set(filePath, buildPromise);
  return buildPromise;
}

function shellQuote(target) {
  const escaped = target.replace(/(["\\$`])/g, "\\$1");
  return `"${escaped}"`;
}

function deactivate() {
  if (outputChannel) {
    outputChannel.dispose();
  }
}

module.exports = {
  activate,
  deactivate,
};
