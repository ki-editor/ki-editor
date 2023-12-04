#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 15
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 12
#define ALIAS_COUNT 0
#define TOKEN_COUNT 5
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 3
#define PRODUCTION_ID_COUNT 1

enum {
  anon_sym_LBRACK_STAR_RBRACK = 1,
  anon_sym_LF = 2,
  anon_sym_PIPE = 3,
  sym_word = 4,
  sym_source_file = 5,
  sym_section = 6,
  sym_header = 7,
  sym_values = 8,
  sym_value = 9,
  aux_sym_source_file_repeat1 = 10,
  aux_sym_values_repeat1 = 11,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_LBRACK_STAR_RBRACK] = "[*]",
  [anon_sym_LF] = "\n",
  [anon_sym_PIPE] = "|",
  [sym_word] = "word",
  [sym_source_file] = "source_file",
  [sym_section] = "section",
  [sym_header] = "header",
  [sym_values] = "values",
  [sym_value] = "value",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_values_repeat1] = "values_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_LBRACK_STAR_RBRACK] = anon_sym_LBRACK_STAR_RBRACK,
  [anon_sym_LF] = anon_sym_LF,
  [anon_sym_PIPE] = anon_sym_PIPE,
  [sym_word] = sym_word,
  [sym_source_file] = sym_source_file,
  [sym_section] = sym_section,
  [sym_header] = sym_header,
  [sym_values] = sym_values,
  [sym_value] = sym_value,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_values_repeat1] = aux_sym_values_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_LBRACK_STAR_RBRACK] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LF] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_PIPE] = {
    .visible = true,
    .named = false,
  },
  [sym_word] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym_section] = {
    .visible = true,
    .named = true,
  },
  [sym_header] = {
    .visible = true,
    .named = true,
  },
  [sym_values] = {
    .visible = true,
    .named = true,
  },
  [sym_value] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
  [aux_sym_values_repeat1] = {
    .visible = false,
    .named = false,
  },
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 6,
  [7] = 7,
  [8] = 8,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 13,
  [14] = 14,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(4);
      if (lookahead == '\n') ADVANCE(6);
      if (lookahead == ' ') SKIP(0)
      if (lookahead == '[') ADVANCE(2);
      if (lookahead == '|') ADVANCE(7);
      END_STATE();
    case 1:
      if (lookahead == ' ') ADVANCE(8);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(9);
      END_STATE();
    case 2:
      if (lookahead == '*') ADVANCE(3);
      END_STATE();
    case 3:
      if (lookahead == ']') ADVANCE(5);
      END_STATE();
    case 4:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 5:
      ACCEPT_TOKEN(anon_sym_LBRACK_STAR_RBRACK);
      END_STATE();
    case 6:
      ACCEPT_TOKEN(anon_sym_LF);
      END_STATE();
    case 7:
      ACCEPT_TOKEN(anon_sym_PIPE);
      END_STATE();
    case 8:
      ACCEPT_TOKEN(sym_word);
      if (lookahead == ' ') ADVANCE(8);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(9);
      END_STATE();
    case 9:
      ACCEPT_TOKEN(sym_word);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(9);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 0},
  [10] = {.lex_state = 1},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 0},
  [13] = {.lex_state = 1},
  [14] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_LBRACK_STAR_RBRACK] = ACTIONS(1),
    [anon_sym_LF] = ACTIONS(1),
    [anon_sym_PIPE] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(11),
    [sym_section] = STATE(2),
    [sym_header] = STATE(6),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(3),
    [anon_sym_LBRACK_STAR_RBRACK] = ACTIONS(5),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 4,
    ACTIONS(5), 1,
      anon_sym_LBRACK_STAR_RBRACK,
    ACTIONS(7), 1,
      ts_builtin_sym_end,
    STATE(6), 1,
      sym_header,
    STATE(4), 2,
      sym_section,
      aux_sym_source_file_repeat1,
  [14] = 3,
    ACTIONS(11), 1,
      anon_sym_PIPE,
    ACTIONS(9), 2,
      ts_builtin_sym_end,
      anon_sym_LBRACK_STAR_RBRACK,
    STATE(5), 2,
      sym_value,
      aux_sym_values_repeat1,
  [26] = 4,
    ACTIONS(13), 1,
      ts_builtin_sym_end,
    ACTIONS(15), 1,
      anon_sym_LBRACK_STAR_RBRACK,
    STATE(6), 1,
      sym_header,
    STATE(4), 2,
      sym_section,
      aux_sym_source_file_repeat1,
  [40] = 3,
    ACTIONS(20), 1,
      anon_sym_PIPE,
    ACTIONS(18), 2,
      ts_builtin_sym_end,
      anon_sym_LBRACK_STAR_RBRACK,
    STATE(5), 2,
      sym_value,
      aux_sym_values_repeat1,
  [52] = 3,
    ACTIONS(11), 1,
      anon_sym_PIPE,
    STATE(9), 1,
      sym_values,
    STATE(3), 2,
      sym_value,
      aux_sym_values_repeat1,
  [63] = 2,
    ACTIONS(25), 1,
      anon_sym_LF,
    ACTIONS(23), 3,
      ts_builtin_sym_end,
      anon_sym_LBRACK_STAR_RBRACK,
      anon_sym_PIPE,
  [72] = 1,
    ACTIONS(27), 3,
      ts_builtin_sym_end,
      anon_sym_LBRACK_STAR_RBRACK,
      anon_sym_PIPE,
  [78] = 1,
    ACTIONS(29), 2,
      ts_builtin_sym_end,
      anon_sym_LBRACK_STAR_RBRACK,
  [83] = 1,
    ACTIONS(31), 1,
      sym_word,
  [87] = 1,
    ACTIONS(33), 1,
      ts_builtin_sym_end,
  [91] = 1,
    ACTIONS(35), 1,
      anon_sym_LF,
  [95] = 1,
    ACTIONS(37), 1,
      sym_word,
  [99] = 1,
    ACTIONS(39), 1,
      anon_sym_PIPE,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(2)] = 0,
  [SMALL_STATE(3)] = 14,
  [SMALL_STATE(4)] = 26,
  [SMALL_STATE(5)] = 40,
  [SMALL_STATE(6)] = 52,
  [SMALL_STATE(7)] = 63,
  [SMALL_STATE(8)] = 72,
  [SMALL_STATE(9)] = 78,
  [SMALL_STATE(10)] = 83,
  [SMALL_STATE(11)] = 87,
  [SMALL_STATE(12)] = 91,
  [SMALL_STATE(13)] = 95,
  [SMALL_STATE(14)] = 99,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [7] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [9] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_values, 1),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [13] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [15] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(10),
  [18] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_values_repeat1, 2),
  [20] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_values_repeat1, 2), SHIFT_REPEAT(13),
  [23] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value, 2),
  [25] = {.entry = {.count = 1, .reusable = true}}, SHIFT(8),
  [27] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value, 3),
  [29] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_section, 2),
  [31] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [33] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [35] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [37] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [39] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_header, 3),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef _WIN32
#define extern __declspec(dllexport)
#endif

extern const TSLanguage *tree_sitter_yard(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
