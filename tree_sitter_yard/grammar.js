const makeQuote = (quote) => ($) =>
  choice(
    prec.left(seq(quote, quote)),
    prec.left(seq(quote, $.expressions, quote))
  );

const makeEnclose = (open, close) => ($) =>
  choice(seq(open, close), seq(open, $.expressions, close));

module.exports = grammar({
  name: "yard",

  extras: ($) => [/\s/], // Ignore whitespace

  rules: {
    // Entry point
    source_file: ($) => repeat($._expression),
    _expression: ($) => choice($._bracket_expression, $._quote_expression),
    expressions: ($) =>
      prec.left(repeat1(choice($._bracket_expression, $._quote_expression))),

    _bracket_expression: ($) =>
      choice(
        $.paren_expression,
        $.brace_expression,
        $.bracket_expression,
        $._base_expression
      ),
    _quote_expression: ($) =>
      choice(
        $.double_quote_expression,
        $.single_quote_expression,
        $.backtick_quote_expression
      ),

    _base_expression: ($) => /[^()\[\]{}`'"]+/,
    paren_expression: makeEnclose("(", ")"),
    brace_expression: makeEnclose("{", "}"),
    bracket_expression: makeEnclose("[", "]"),
    double_quote_expression: makeQuote('"'),
    single_quote_expression: makeQuote("'"),
    backtick_quote_expression: makeQuote("`"),
  },
});
