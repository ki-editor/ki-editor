#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 14
#define LARGE_STATE_COUNT 6
#define SYMBOL_COUNT 11
#define ALIAS_COUNT 0
#define TOKEN_COUNT 4
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 3
#define PRODUCTION_ID_COUNT 1

enum {
  anon_sym_LF = 1,
  anon_sym_ = 2,
  sym_word = 3,
  sym_source_file = 4,
  sym_section = 5,
  sym_header = 6,
  sym_values = 7,
  sym_value = 8,
  aux_sym_source_file_repeat1 = 9,
  aux_sym_values_repeat1 = 10,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_LF] = "\n",
  [anon_sym_] = "    ",
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
  [anon_sym_LF] = anon_sym_LF,
  [anon_sym_] = anon_sym_,
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
  [anon_sym_LF] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_] = {
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
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(7);
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(12);
      if (lookahead != 0) ADVANCE(14);
      END_STATE();
    case 1:
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(9);
      END_STATE();
    case 2:
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(1);
      END_STATE();
    case 3:
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(2);
      END_STATE();
    case 4:
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(3);
      END_STATE();
    case 5:
      if (eof) ADVANCE(7);
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') SKIP(5)
      END_STATE();
    case 6:
      if (eof) ADVANCE(7);
      if (lookahead == '\n') ADVANCE(8);
      if (lookahead == ' ') ADVANCE(13);
      if (lookahead != 0) ADVANCE(14);
      END_STATE();
    case 7:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 8:
      ACCEPT_TOKEN(anon_sym_LF);
      END_STATE();
    case 9:
      ACCEPT_TOKEN(anon_sym_);
      if (lookahead == ' ') ADVANCE(9);
      END_STATE();
    case 10:
      ACCEPT_TOKEN(sym_word);
      if (lookahead == ' ') ADVANCE(9);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(14);
      END_STATE();
    case 11:
      ACCEPT_TOKEN(sym_word);
      if (lookahead == ' ') ADVANCE(10);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(14);
      END_STATE();
    case 12:
      ACCEPT_TOKEN(sym_word);
      if (lookahead == ' ') ADVANCE(11);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(14);
      END_STATE();
    case 13:
      ACCEPT_TOKEN(sym_word);
      if (lookahead == ' ') ADVANCE(13);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(14);
      END_STATE();
    case 14:
      ACCEPT_TOKEN(sym_word);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(14);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0},
  [1] = {.lex_state = 6},
  [2] = {.lex_state = 6},
  [3] = {.lex_state = 6},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 4},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 6},
  [10] = {.lex_state = 5},
  [11] = {.lex_state = 6},
  [12] = {.lex_state = 5},
  [13] = {.lex_state = 5},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_LF] = ACTIONS(3),
    [anon_sym_] = ACTIONS(1),
    [sym_word] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(10),
    [sym_section] = STATE(2),
    [sym_header] = STATE(13),
    [aux_sym_source_file_repeat1] = STATE(2),
    [anon_sym_LF] = ACTIONS(3),
    [sym_word] = ACTIONS(5),
  },
  [2] = {
    [sym_section] = STATE(3),
    [sym_header] = STATE(13),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(7),
    [anon_sym_LF] = ACTIONS(3),
    [sym_word] = ACTIONS(5),
  },
  [3] = {
    [sym_section] = STATE(3),
    [sym_header] = STATE(13),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(9),
    [anon_sym_LF] = ACTIONS(3),
    [sym_word] = ACTIONS(11),
  },
  [4] = {
    [sym_value] = STATE(5),
    [aux_sym_values_repeat1] = STATE(5),
    [ts_builtin_sym_end] = ACTIONS(14),
    [anon_sym_LF] = ACTIONS(3),
    [anon_sym_] = ACTIONS(16),
    [sym_word] = ACTIONS(18),
  },
  [5] = {
    [sym_value] = STATE(5),
    [aux_sym_values_repeat1] = STATE(5),
    [ts_builtin_sym_end] = ACTIONS(20),
    [anon_sym_LF] = ACTIONS(3),
    [anon_sym_] = ACTIONS(22),
    [sym_word] = ACTIONS(25),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 4,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(27), 1,
      anon_sym_,
    STATE(9), 1,
      sym_values,
    STATE(4), 2,
      sym_value,
      aux_sym_values_repeat1,
  [14] = 3,
    ACTIONS(29), 1,
      ts_builtin_sym_end,
    ACTIONS(31), 1,
      anon_sym_LF,
    ACTIONS(33), 2,
      anon_sym_,
      sym_word,
  [25] = 3,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(35), 1,
      ts_builtin_sym_end,
    ACTIONS(37), 2,
      anon_sym_,
      sym_word,
  [36] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(39), 2,
      ts_builtin_sym_end,
      sym_word,
  [44] = 2,
    ACTIONS(41), 1,
      ts_builtin_sym_end,
    ACTIONS(43), 1,
      anon_sym_LF,
  [51] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(45), 1,
      sym_word,
  [58] = 1,
    ACTIONS(47), 1,
      anon_sym_LF,
  [62] = 1,
    ACTIONS(49), 1,
      anon_sym_LF,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(6)] = 0,
  [SMALL_STATE(7)] = 14,
  [SMALL_STATE(8)] = 25,
  [SMALL_STATE(9)] = 36,
  [SMALL_STATE(10)] = 44,
  [SMALL_STATE(11)] = 51,
  [SMALL_STATE(12)] = 58,
  [SMALL_STATE(13)] = 62,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [7] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [9] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [11] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(12),
  [14] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_values, 1),
  [16] = {.entry = {.count = 1, .reusable = false}}, SHIFT(11),
  [18] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_values, 1),
  [20] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_values_repeat1, 2),
  [22] = {.entry = {.count = 2, .reusable = false}}, REDUCE(aux_sym_values_repeat1, 2), SHIFT_REPEAT(11),
  [25] = {.entry = {.count = 1, .reusable = false}}, REDUCE(aux_sym_values_repeat1, 2),
  [27] = {.entry = {.count = 1, .reusable = true}}, SHIFT(11),
  [29] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value, 2),
  [31] = {.entry = {.count = 1, .reusable = false}}, SHIFT(8),
  [33] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_value, 2),
  [35] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value, 3),
  [37] = {.entry = {.count = 1, .reusable = false}}, REDUCE(sym_value, 3),
  [39] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_section, 3),
  [41] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [43] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [45] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [47] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_header, 1),
  [49] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
};

#ifdef __cplusplus
extern "C" {
#endif
#ifdef _WIN32
#define extern __declspec(dllexport)
#endif

extern const TSLanguage *tree_sitter_quickfix(void) {
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
