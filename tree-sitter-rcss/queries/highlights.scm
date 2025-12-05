; Comments
((comment) @comment)

; Selectors and class/id names
(selector (identifier) @selector)
(class_selector (identifier) @type)
(id_selector (identifier) @type)

; Block selectors (e.g., screen(@md), dark)
(at_rule
  (identifier) @keyword
  (arguments)? @parameter)

; Properties
(declaration
  (property_name) @property
  (value) @string)

; Presets (%base-16, %dark reading)
((preset_directive) @keyword)

; Variables ($accent, block vars)
((variable_name) @variable)

; Tokens (@blue-500, gradient @names)
((token) @constant)

; Numbers and dimensions
((number) @number)
((dimension) @number)

; Functions and shorthands (translate(), scale(), shadow:)
((function_name) @function)

; Strings / URLs
((string) @string)
((url) @string.special)
