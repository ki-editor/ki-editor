#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 18
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 14
#define ALIAS_COUNT 0
#define TOKEN_COUNT 6
#define EXTERNAL_TOKEN_COUNT 0
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 3
#define PRODUCTION_ID_COUNT 1

enum {
  anon_sym_ = 1,
  anon_sym_2 = 2,
  anon_sym_LF = 3,
  anon_sym_3 = 4,
  sym_word = 5,
  sym_source_file = 6,
  sym_section = 7,
  sym_header = 8,
  sym_values = 9,
  sym_value = 10,
  sym_lastValue = 11,
  aux_sym_source_file_repeat1 = 12,
  aux_sym_values_repeat1 = 13,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_] = "■┬",
  [anon_sym_2] = "├",
  [anon_sym_LF] = "\n",
  [anon_sym_3] = "└",
  [sym_word] = "word",
  [sym_source_file] = "source_file",
  [sym_section] = "section",
  [sym_header] = "header",
  [sym_values] = "values",
  [sym_value] = "value",
  [sym_lastValue] = "lastValue",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
  [aux_sym_values_repeat1] = "values_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_] = anon_sym_,
  [anon_sym_2] = anon_sym_2,
  [anon_sym_LF] = anon_sym_LF,
  [anon_sym_3] = anon_sym_3,
  [sym_word] = sym_word,
  [sym_source_file] = sym_source_file,
  [sym_section] = sym_section,
  [sym_header] = sym_header,
  [sym_values] = sym_values,
  [sym_value] = sym_value,
  [sym_lastValue] = sym_lastValue,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
  [aux_sym_values_repeat1] = aux_sym_values_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_2] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LF] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_3] = {
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
  [sym_lastValue] = {
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
  [15] = 15,
  [16] = 16,
  [17] = 17,
};

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(3);
      if (lookahead == '\n') ADVANCE(6);
      if (lookahead == ' ') SKIP(0)
      if (lookahead == 9492) ADVANCE(7);
      if (lookahead == 9500) ADVANCE(5);
      if (lookahead == 9632) ADVANCE(2);
      END_STATE();
    case 1:
      if (lookahead == '\n') ADVANCE(6);
      if (lookahead == ' ') ADVANCE(8);
      if (lookahead != 0) ADVANCE(9);
      END_STATE();
    case 2:
      if (lookahead == 9516) ADVANCE(4);
      END_STATE();
    case 3:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 4:
      ACCEPT_TOKEN(anon_sym_);
      END_STATE();
    case 5:
      ACCEPT_TOKEN(anon_sym_2);
      END_STATE();
    case 6:
      ACCEPT_TOKEN(anon_sym_LF);
      END_STATE();
    case 7:
      ACCEPT_TOKEN(anon_sym_3);
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
  [10] = {.lex_state = 0},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 0},
  [13] = {.lex_state = 1},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 1},
  [16] = {.lex_state = 1},
  [17] = {.lex_state = 0},
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_] = ACTIONS(1),
    [anon_sym_2] = ACTIONS(1),
    [anon_sym_LF] = ACTIONS(3),
    [anon_sym_3] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(14),
    [sym_section] = STATE(3),
    [sym_header] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(3),
    [ts_builtin_sym_end] = ACTIONS(5),
    [anon_sym_] = ACTIONS(7),
    [anon_sym_LF] = ACTIONS(3),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 6,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(9), 1,
      anon_sym_2,
    ACTIONS(11), 1,
      anon_sym_3,
    STATE(8), 1,
      sym_values,
    STATE(9), 1,
      sym_lastValue,
    STATE(4), 2,
      sym_value,
      aux_sym_values_repeat1,
  [20] = 5,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(7), 1,
      anon_sym_,
    ACTIONS(13), 1,
      ts_builtin_sym_end,
    STATE(2), 1,
      sym_header,
    STATE(5), 2,
      sym_section,
      aux_sym_source_file_repeat1,
  [37] = 5,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(9), 1,
      anon_sym_2,
    ACTIONS(11), 1,
      anon_sym_3,
    STATE(11), 1,
      sym_lastValue,
    STATE(6), 2,
      sym_value,
      aux_sym_values_repeat1,
  [54] = 5,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(15), 1,
      ts_builtin_sym_end,
    ACTIONS(17), 1,
      anon_sym_,
    STATE(2), 1,
      sym_header,
    STATE(5), 2,
      sym_section,
      aux_sym_source_file_repeat1,
  [71] = 4,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(20), 1,
      anon_sym_2,
    ACTIONS(23), 1,
      anon_sym_3,
    STATE(6), 2,
      sym_value,
      aux_sym_values_repeat1,
  [85] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(25), 2,
      anon_sym_2,
      anon_sym_3,
  [93] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(27), 2,
      ts_builtin_sym_end,
      anon_sym_,
  [101] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(29), 2,
      ts_builtin_sym_end,
      anon_sym_,
  [109] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(31), 2,
      ts_builtin_sym_end,
      anon_sym_,
  [117] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(33), 2,
      ts_builtin_sym_end,
      anon_sym_,
  [125] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(35), 2,
      anon_sym_2,
      anon_sym_3,
  [133] = 2,
    ACTIONS(37), 1,
      anon_sym_LF,
    ACTIONS(39), 1,
      sym_word,
  [140] = 2,
    ACTIONS(3), 1,
      anon_sym_LF,
    ACTIONS(41), 1,
      ts_builtin_sym_end,
  [147] = 2,
    ACTIONS(37), 1,
      anon_sym_LF,
    ACTIONS(43), 1,
      sym_word,
  [154] = 2,
    ACTIONS(37), 1,
      anon_sym_LF,
    ACTIONS(45), 1,
      sym_word,
  [161] = 1,
    ACTIONS(47), 1,
      anon_sym_LF,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(2)] = 0,
  [SMALL_STATE(3)] = 20,
  [SMALL_STATE(4)] = 37,
  [SMALL_STATE(5)] = 54,
  [SMALL_STATE(6)] = 71,
  [SMALL_STATE(7)] = 85,
  [SMALL_STATE(8)] = 93,
  [SMALL_STATE(9)] = 101,
  [SMALL_STATE(10)] = 109,
  [SMALL_STATE(11)] = 117,
  [SMALL_STATE(12)] = 125,
  [SMALL_STATE(13)] = 133,
  [SMALL_STATE(14)] = 140,
  [SMALL_STATE(15)] = 147,
  [SMALL_STATE(16)] = 154,
  [SMALL_STATE(17)] = 161,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [9] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [11] = {.entry = {.count = 1, .reusable = true}}, SHIFT(16),
  [13] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [15] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [17] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(13),
  [20] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_values_repeat1, 2), SHIFT_REPEAT(15),
  [23] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_values_repeat1, 2),
  [25] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_header, 2),
  [27] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_section, 2),
  [29] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_values, 1),
  [31] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_lastValue, 2),
  [33] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_values, 2),
  [35] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_value, 3),
  [37] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [39] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [41] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [43] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [45] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [47] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
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
