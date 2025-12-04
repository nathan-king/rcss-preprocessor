/**
 * @file A Tree-sitter grammar for the RCSS stylesheet language.
 * @author Nathan King <hello@nathanking.io>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "rcss",

  rules: {
    // TODO: add the actual grammar rules
    source_file: $ => "hello"
  }
});
