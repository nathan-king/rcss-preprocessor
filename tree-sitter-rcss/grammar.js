module.exports = grammar({
  name: "rcss",

  rules: {
    source_file: ($) => repeat($.rule),

    rule: ($) => seq($.selector, $.block),

    selector: ($) =>
      seq(choice($.class_selector, $.id_selector, $.element_selector)),

    class_selector: ($) => seq(".", /[a-zA-Z_][a-zA-Z0-9_-]*/),

    id_selector: ($) => seq("#", /[a-zA-Z_][a-zA-Z0-9_-]*/),

    element_selector: ($) => /[a-zA-Z_][a-zA-Z0-9_-]*/,

    block: ($) => seq("{", repeat($.declaration), "}"),

    declaration: ($) => seq($.property, ":", $.value, ";"),

    property: ($) => /[a-zA-Z-]+/,

    value: ($) => repeat1(choice($.token, $.number, $.color, $.identifier)),

    token: ($) => /@[a-zA-Z0-9_-]+(\/[0-9.]+)?/,

    number: ($) => /\d+(\.\d+)?(rem|px|%|em)?/,

    color: ($) =>
      choice(/#[0-9a-fA-F]{3,6}/, /oklch\([^)]*\)/, /rgba?\([^)]*\)/),

    identifier: ($) => /[a-zA-Z0-9_-]+/,
  },
});
