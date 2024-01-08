x = {
	args = { "--stdio" },
	cmd = "/Users/wongjiahau/.local/share/nvim/mason/bin/typescript-language-server",
	extra = { cwd = "/Users/wongjiahau/repos/editor-idea" },
}

x = {
	id = 1,
	jsonrpc = "2.0",
	method = "initialize",
	params = {
		capabilities = {
			textDocument = {
				callHierarchy = { dynamicRegistration = false },
				codeAction = {
					codeActionLiteralSupport = {
						codeActionKind = {
							valueSet = {
								"",
								"quickfix",
								"refactor",
								"refactor.extract",
								"refactor.inline",
								"refactor.rewrite",
								"source",
								"source.organizeImports",
							},
						},
					},
					dataSupport = true,
					dynamicRegistration = false,
					isPreferredSupport = true,
					resolveSupport = { properties = { "edit" } },
				},
				completion = {
					completionItem = {
						commitCharactersSupport = false,
						deprecatedSupport = false,
						documentationFormat = { "markdown", "plaintext" },
						preselectSupport = false,
						snippetSupport = false,
					},
					completionItemKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
						},
					},
					contextSupport = false,
					dynamicRegistration = false,
				},
				declaration = { linkSupport = true },
				definition = { linkSupport = true },
				documentHighlight = { dynamicRegistration = false },
				documentSymbol = {
					dynamicRegistration = false,
					hierarchicalDocumentSymbolSupport = true,
					symbolKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
							26,
						},
					},
				},
				hover = {
					contentFormat = { "markdown", "plaintext" },
					dynamicRegistration = false,
				},
				implementation = { linkSupport = true },
				publishDiagnostics = {
					relatedInformation = true,
					tagSupport = { valueSet = { 1, 2 } },
				},
				references = { dynamicRegistration = false },
				rename = { dynamicRegistration = false, prepareSupport = true },
				semanticTokens = {
					augmentsSyntaxTokens = true,
					dynamicRegistration = false,
					formats = { "relative" },
					multilineTokenSupport = false,
					overlappingTokenSupport = true,
					requests = {
						full = { delta = true },
						range = false,
					},
					serverCancelSupport = false,
					tokenModifiers = {
						"declaration",
						"definition",
						"readonly",
						"static",
						"deprecated",
						"abstract",
						"async",
						"modification",
						"documentation",
						"defaultLibrary",
					},
					tokenTypes = {
						"namespace",
						"type",
						"class",
						"enum",
						"interface",
						"struct",
						"typeParameter",
						"parameter",
						"variable",
						"property",
						"enumMember",
						"event",
						"function",
						"method",
						"macro",
						"keyword",
						"modifier",
						"comment",
						"string",
						"number",
						"regexp",
						"operator",
						"decorator",
					},
				},
				signatureHelp = {
					dynamicRegistration = false,
					signatureInformation = {
						activeParameterSupport = true,
						documentationFormat = { "markdown", "plaintext" },
						parameterInformation = { labelOffsetSupport = true },
					},
				},
				synchronization = {
					didSave = true,
					dynamicRegistration = false,
					willSave = true,
					willSaveWaitUntil = true,
				},
				typeDefinition = { linkSupport = true },
			},
			window = {
				showDocument = { support = true },
				showMessage = {
					messageActionItem = { additionalPropertiesSupport = false },
				},
				workDoneProgress = true,
			},
			workspace = {
				applyEdit = true,
				configuration = true,
				didChangeWatchedFiles = {
					dynamicRegistration = false,
					relativePatternSupport = true,
				},
				semanticTokens = { refreshSupport = true },
				symbol = {
					dynamicRegistration = false,
					hierarchicalWorkspaceSymbolSupport = true,
					symbolKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
							26,
						},
					},
				},
				workspaceEdit = { resourceOperations = { "rename", "create", "delete" } },
				workspaceFolders = true,
			},
		},
		clientInfo = { name = "Neovim", version = "0.9.0" },
		initializationOptions = {
			hostInfo = "neovim",
			preferences = { importModuleSpecifierPreference = "relative" },
		},
		processId = 49814,
		rootPath = "/Users/wongjiahau/repos/editor-idea",
		rootUri = "file:///Users/wongjiahau/repos/editor-idea",
		trace = "off",
		workspaceFolders = {
			{
				name = "/Users/wongjiahau/repos/editor-idea",
				uri = "file:///Users/wongjiahau/repos/editor-idea",
			},
		},
	},
}

x = {
	jsonrpc = "2.0",
	method = "window/logMessage",
	params = {
		message = 'Using Typescript version (bundled) 5.0.4 from path "/Users/wongjiahau/.local/share/nvim/mason/packages/typescript-language-server/node_modules/typescript/lib/tsserver.js"',
		type = 3,
	},
}

x = {
	id = 0,
	jsonrpc = "2.0",
	method = "window/workDoneProgress/create",
	params = { token = "d02e3d15-6834-4e04-98cd-e4360a8b60c5" },
}

x = {
	id = 1,
	jsonrpc = "2.0",
	result = {
		capabilities = {
			callHierarchyProvider = true,
			codeActionProvider = {
				codeActionKinds = {
					"source.fixAll.ts",
					"source.removeUnused.ts",
					"source.addMissingImports.ts",
					"source.organizeImports.ts",
					"source.removeUnusedImports.ts",
					"source.sortImports.ts",
					"quickfix",
					"refactor",
				},
			},
			completionProvider = {
				resolveProvider = true,
				triggerCharacters = { ".", '"', "'", "/", "@", "<" },
			},
			definitionProvider = true,
			documentFormattingProvider = true,
			documentHighlightProvider = true,
			documentRangeFormattingProvider = true,
			documentSymbolProvider = true,
			executeCommandProvider = {
				commands = {
					"_typescript.applyWorkspaceEdit",
					"_typescript.applyCodeAction",
					"_typescript.applyRefactoring",
					"_typescript.configurePlugin",
					"_typescript.organizeImports",
					"_typescript.applyRenameFile",
					"_typescript.goToSourceDefinition",
				},
			},
			foldingRangeProvider = true,
			hoverProvider = true,
			implementationProvider = true,
			inlayHintProvider = true,
			referencesProvider = true,
			renameProvider = { prepareProvider = true },
			selectionRangeProvider = true,
			semanticTokensProvider = {
				full = true,
				legend = {
					tokenModifiers = { "declaration", "static", "async", "readonly", "defaultLibrary", "local" },
					tokenTypes = {
						"class",
						"enum",
						"interface",
						"namespace",
						"typeParameter",
						"type",
						"parameter",
						"variable",
						"enumMember",
						"property",
						"function",
						"member",
					},
				},
				range = true,
			},
			signatureHelpProvider = {
				retriggerCharacters = { ")" },
				triggerCharacters = { "(", ",", "<" },
			},
			textDocumentSync = 2,
			typeDefinitionProvider = true,
			workspace = {
				fileOperations = {
					willRename = {
						filters = {
							{
								pattern = {
									glob = "**/*.{ts,js,jsx,tsx,mjs,mts,cjs,cts}",
									matches = "file",
								},
								scheme = "file",
							},
						},
					},
				},
			},
			workspaceSymbolProvider = true,
		},
	},
}

x = {
	args = { "/Users/wongjiahau/.config/nvim/plugged/copilot.vim/dist/agent.js" },
	cmd = "node",
	extra = { cwd = "/Users/wongjiahau" },
}

x = {
	id = 1,
	jsonrpc = "2.0",
	method = "initialize",
	params = {
		capabilities = {
			textDocument = {
				callHierarchy = { dynamicRegistration = false },
				codeAction = {
					codeActionLiteralSupport = {
						codeActionKind = {
							valueSet = {
								"",
								"quickfix",
								"refactor",
								"refactor.extract",
								"refactor.inline",
								"refactor.rewrite",
								"source",
								"source.organizeImports",
							},
						},
					},
					dataSupport = true,
					dynamicRegistration = false,
					isPreferredSupport = true,
					resolveSupport = { properties = { "edit" } },
				},
				completion = {
					completionItem = {
						commitCharactersSupport = false,
						deprecatedSupport = false,
						documentationFormat = { "markdown", "plaintext" },
						preselectSupport = false,
						snippetSupport = false,
					},
					completionItemKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
						},
					},
					contextSupport = false,
					dynamicRegistration = false,
				},
				declaration = { linkSupport = true },
				definition = { linkSupport = true },
				documentHighlight = { dynamicRegistration = false },
				documentSymbol = {
					dynamicRegistration = false,
					hierarchicalDocumentSymbolSupport = true,
					symbolKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
							26,
						},
					},
				},
				hover = {
					contentFormat = { "markdown", "plaintext" },
					dynamicRegistration = false,
				},
				implementation = { linkSupport = true },
				publishDiagnostics = {
					relatedInformation = true,
					tagSupport = { valueSet = { 1, 2 } },
				},
				references = { dynamicRegistration = false },
				rename = { dynamicRegistration = false, prepareSupport = true },
				semanticTokens = {
					augmentsSyntaxTokens = true,
					dynamicRegistration = false,
					formats = { "relative" },
					multilineTokenSupport = false,
					overlappingTokenSupport = true,
					requests = {
						full = { delta = true },
						range = false,
					},
					serverCancelSupport = false,
					tokenModifiers = {
						"declaration",
						"definition",
						"readonly",
						"static",
						"deprecated",
						"abstract",
						"async",
						"modification",
						"documentation",
						"defaultLibrary",
					},
					tokenTypes = {
						"namespace",
						"type",
						"class",
						"enum",
						"interface",
						"struct",
						"typeParameter",
						"parameter",
						"variable",
						"property",
						"enumMember",
						"event",
						"function",
						"method",
						"macro",
						"keyword",
						"modifier",
						"comment",
						"string",
						"number",
						"regexp",
						"operator",
						"decorator",
					},
				},
				signatureHelp = {
					dynamicRegistration = false,
					signatureInformation = {
						activeParameterSupport = true,
						documentationFormat = { "markdown", "plaintext" },
						parameterInformation = { labelOffsetSupport = true },
					},
				},
				synchronization = {
					didSave = true,
					dynamicRegistration = false,
					willSave = true,
					willSaveWaitUntil = true,
				},
				typeDefinition = { linkSupport = true },
			},
			window = {
				showDocument = { support = true },
				showMessage = {
					messageActionItem = { additionalPropertiesSupport = false },
				},
				workDoneProgress = true,
			},
			workspace = {
				applyEdit = true,
				configuration = true,
				didChangeWatchedFiles = {
					dynamicRegistration = false,
					relativePatternSupport = true,
				},
				semanticTokens = { refreshSupport = true },
				symbol = {
					dynamicRegistration = false,
					hierarchicalWorkspaceSymbolSupport = true,
					symbolKind = {
						valueSet = {
							1,
							2,
							3,
							4,
							5,
							6,
							7,
							8,
							9,
							10,
							11,
							12,
							13,
							14,
							15,
							16,
							17,
							18,
							19,
							20,
							21,
							22,
							23,
							24,
							25,
							26,
						},
					},
				},
				workspaceEdit = { resourceOperations = { "rename", "create", "delete" } },
				workspaceFolders = true,
			},
		},
		clientInfo = { name = "Neovim", version = "0.9.0" },
		processId = 49814,
		rootPath = vim.NIL,
		rootUri = vim.NIL,
		trace = "off",
		workspaceFolders = vim.NIL,
	},
}

x = { result = vim.NIL, status = true }

x = { id = 0, jsonrpc = "2.0", result = vim.NIL }

x = { jsonrpc = "2.0", method = "initialized", params = vim.empty_dict() }

x = {
	server_capabilities = {
		callHierarchyProvider = true,
		codeActionProvider = {
			codeActionKinds = {
				"source.fixAll.ts",
				"source.removeUnused.ts",
				"source.addMissingImports.ts",
				"source.organizeImports.ts",
				"source.removeUnusedImports.ts",
				"source.sortImports.ts",
				"quickfix",
				"refactor",
			},
		},
		completionProvider = {
			resolveProvider = true,
			triggerCharacters = { ".", '"', "'", "/", "@", "<" },
		},
		definitionProvider = true,
		documentFormattingProvider = true,
		documentHighlightProvider = true,
		documentRangeFormattingProvider = true,
		documentSymbolProvider = true,
		executeCommandProvider = {
			commands = {
				"_typescript.applyWorkspaceEdit",
				"_typescript.applyCodeAction",
				"_typescript.applyRefactoring",
				"_typescript.configurePlugin",
				"_typescript.organizeImports",
				"_typescript.applyRenameFile",
				"_typescript.goToSourceDefinition",
			},
		},
		foldingRangeProvider = true,
		hoverProvider = true,
		implementationProvider = true,
		inlayHintProvider = true,
		referencesProvider = true,
		renameProvider = { prepareProvider = true },
		selectionRangeProvider = true,
		semanticTokensProvider = {
			full = true,
			legend = {
				tokenModifiers = { "declaration", "static", "async", "readonly", "defaultLibrary", "local" },
				tokenTypes = {
					"class",
					"enum",
					"interface",
					"namespace",
					"typeParameter",
					"type",
					"parameter",
					"variable",
					"enumMember",
					"property",
					"function",
					"member",
				},
			},
			range = true,
		},
		signatureHelpProvider = { retriggerCharacters = { ")" }, triggerCharacters = { "(", ",", "<" } },
		textDocumentSync = {
			change = 2,
			openClose = true,
			save = { includeText = false },
			willSave = false,
			willSaveWaitUntil = false,
		},
		typeDefinitionProvider = true,
		workspace = {
			fileOperations = {
				willRename = {
					filters = {
						{
							pattern = {
								glob = "**/*.{ts,js,jsx,tsx,mjs,mts,cjs,cts}",
								matches = "file",
							},
							scheme = "file",
						},
					},
				},
			},
		},
		workspaceSymbolProvider = true,
	},
}

x = {
	jsonrpc = "2.0",
	method = "textDocument/didOpen",
	params = {
		textDocument = {
			languageId = "typescript",
			text = "const hello = 1;\nconsole.log(hello2);\n",
			uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
			version = 0,
		},
	},
}

x = {
	jsonrpc = "2.0",
	method = "$/progress",
	params = {
		token = "d02e3d15-6834-4e04-98cd-e4360a8b60c5",
		value = { kind = "begin", title = "Initializing JS/TS language features…" },
	},
}

x = { jsonrpc = "2.0", method = "$/typescriptVersion", params = { source = "bundled", version = "5.0.4" } }

x = { textDocument = { uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts" } }

x = {
	id = 2,
	jsonrpc = "2.0",
	method = "textDocument/semanticTokens/full",
	params = { textDocument = { uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts" } },
}

x = {
	jsonrpc = "2.0",
	method = "LogMessage",
	params = {
		extra = { "Agent service starting" },
		level = 0,
		message = "[DEBUG] [agent] [2024-01-01T07:42:02.851Z] Agent service starting",
		metadataStr = "[DEBUG] [agent] [2024-01-01T07:42:02.851Z]",
	},
}

x = {
	id = 0,
	jsonrpc = "2.0",
	method = "client/registerCapability",
	params = {
		registrations = {
			{
				id = "80e165fa-ab86-41f7-a642-994fa69fd51d",
				method = "workspace/didChangeWorkspaceFolders",
				registerOptions = vim.empty_dict(),
			},
		},
	},
}

x = { result = vim.NIL, status = true }

x = { id = 0, jsonrpc = "2.0", result = vim.NIL }

x = {
	id = 1,
	jsonrpc = "2.0",
	result = {
		capabilities = {
			textDocumentSync = { change = 2, openClose = true },
			workspace = {
				workspaceFolders = { changeNotifications = true, supported = true },
			},
		},
	},
}

x = { jsonrpc = "2.0", method = "initialized", params = vim.empty_dict() }

x = {
	server_capabilities = {
		textDocumentSync = { change = 2, openClose = true },
		workspace = { workspaceFolders = { changeNotifications = true, supported = true } },
	},
}

x = {
	jsonrpc = "2.0",
	method = "textDocument/didOpen",
	params = {
		textDocument = {
			languageId = "typescript",
			text = "const hello = 1;\nconsole.log(hello2);\n",
			uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
			version = 0,
		},
	},
}

x = {
	editorConfiguration = {
		disabledLanguages = {
			{ languageId = "." },
			{ languageId = "cvs" },
			{ languageId = "gitcommit" },
			{ languageId = "gitrebase" },
			{ languageId = "help" },
			{ languageId = "hgcommit" },
			{ languageId = "markdown" },
			{ languageId = "svn" },
			{ languageId = "yaml" },
		},
		enableAutoCompletions = true,
	},
	editorInfo = { name = "Neovim", version = "0.9.0 + Node.js 16.15.1" },
	editorPluginInfo = { name = "copilot.vim", version = "1.9.1" },
}

x = {
	id = 2,
	jsonrpc = "2.0",
	method = "setEditorInfo",
	params = {
		editorConfiguration = {
			disabledLanguages = {
				{ languageId = "." },
				{ languageId = "cvs" },
				{ languageId = "gitcommit" },
				{ languageId = "gitrebase" },
				{ languageId = "help" },
				{ languageId = "hgcommit" },
				{ languageId = "markdown" },
				{ languageId = "svn" },
				{ languageId = "yaml" },
			},
			enableAutoCompletions = true,
		},
		editorInfo = { name = "Neovim", version = "0.9.0 + Node.js 16.15.1" },
		editorPluginInfo = { name = "copilot.vim", version = "1.9.1" },
	},
}

x = {
	jsonrpc = "2.0",
	method = "LogMessage",
	params = {
		extra = { "Telemetry initialized" },
		level = 0,
		message = "[DEBUG] [agent] [2024-01-01T07:42:02.863Z] Telemetry initialized",
		metadataStr = "[DEBUG] [agent] [2024-01-01T07:42:02.863Z]",
	},
}

x = { id = 2, jsonrpc = "2.0", result = "OK" }

x = { id = 2, jsonrpc = "2.0", result = { data = { 0, 6, 5, 7, 9, 1, 0, 7, 7, 16, 0, 8, 3, 11, 16 } } }

x = {
	jsonrpc = "2.0",
	method = "$/progress",
	params = { token = "d02e3d15-6834-4e04-98cd-e4360a8b60c5", value = { kind = "end" } },
}

x = {
	jsonrpc = "2.0",
	method = "textDocument/publishDiagnostics",
	params = {
		diagnostics = {
			{
				code = 2552,
				message = "Cannot find name 'hello2'. Did you mean 'hello'?",
				range = {
					["end"] = { character = 18, line = 1 },
					start = { character = 12, line = 1 },
				},
				relatedInformation = {
					{
						location = {
							range = {
								["end"] = { character = 11, line = 0 },
								start = { character = 6, line = 0 },
							},
							uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
						},
						message = "'hello' is declared here.",
					},
				},
				severity = 1,
				source = "typescript",
				tags = {},
			},
		},
		uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
	},
}

x = {
	context = {
		diagnostics = {
			{
				code = 2552,
				message = "Cannot find name 'hello2'. Did you mean 'hello'?",
				range = {
					["end"] = { character = 18, line = 1 },
					start = { character = 12, line = 1 },
				},
				relatedInformation = {
					{
						location = {
							range = {
								["end"] = { character = 11, line = 0 },
								start = { character = 6, line = 0 },
							},
							uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
						},
						message = "'hello' is declared here.",
					},
				},
				severity = 1,
				source = "typescript",
			},
		},
	},
	range = { ["end"] = { character = 12, line = 1 }, start = nil },
	textDocument = { uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts" },
}

x = {
	id = 3,
	jsonrpc = "2.0",
	method = "textDocument/codeAction",
	params = {
		context = {
			diagnostics = {
				{
					code = 2552,
					message = "Cannot find name 'hello2'. Did you mean 'hello'?",
					range = {
						["end"] = { character = 18, line = 1 },
						start = { character = 12, line = 1 },
					},
					relatedInformation = {
						{
							location = {
								range = {
									["end"] = {
										character = 11,
										line = 0,
									},
									start = {
										character = 6,
										line = 0,
									},
								},
								uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
							},
							message = "'hello' is declared here.",
						},
					},
					severity = 1,
					source = "typescript",
				},
			},
		},
		range = { ["end"] = { character = 12, line = 1 }, start = nil },
		textDocument = { uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts" },
	},
}

x = {
	id = 3,
	jsonrpc = "2.0",
	result = {
		{
			command = {
				arguments = {
					{
						documentChanges = {
							{
								edits = {
									{
										newText = "hello",
										range = {
											["end"] = {
												character = 18,
												line = 1,
											},
											start = {
												character = 12,
												line = 1,
											},
										},
									},
								},
								textDocument = {
									uri = "file:///Users/wongjiahau/repos/editor-idea/hello.ts",
									version = 0,
								},
							},
						},
					},
				},
				command = "_typescript.applyWorkspaceEdit",
				title = "Change spelling to 'hello'",
			},
			kind = "quickfix",
			title = "Change spelling to 'hello'",
		},
	},
}

x = {
	{
		_on_attach = nil,
		attached_buffers = { true },
		cancel_request = nil,
		commands = {},
		config = {
			{
				textDocument = {
					completion = {
						completionItem = {
							commitCharactersSupport = true,
							deprecatedSupport = true,
							insertReplaceSupport = true,
							insertTextModeSupport = { valueSet = { 1, 2 } },
							labelDetailsSupport = true,
							preselectSupport = true,
							resolveSupport = {
								properties = {
									"documentation",
									"detail",
									"additionalTextEdits",
									"sortText",
									"filterText",
									"insertText",
									"textEdit",
									"insertTextFormat",
									"insertTextMode",
								},
							},
							snippetSupport = true,
							tagSupport = { valueSet = { 1 } },
						},
						completionList = {
							itemDefaults = {
								"commitCharacters",
								"editRange",
								"insertTextFormat",
								"insertTextMode",
								"data",
							},
						},
						contextSupport = true,
						dynamicRegistration = false,
						insertTextMode = 1,
					},
				},
			},
			autostart = true,
			capabilities = {
				textDocument = {
					callHierarchy = { dynamicRegistration = false },
					codeAction = {
						codeActionLiteralSupport = {
							codeActionKind = {
								valueSet = {
									"",
									"quickfix",
									"refactor",
									"refactor.extract",
									"refactor.inline",
									"refactor.rewrite",
									"source",
									"source.organizeImports",
								},
							},
						},
						dataSupport = true,
						dynamicRegistration = false,
						isPreferredSupport = true,
						resolveSupport = { properties = { "edit" } },
					},
					completion = {
						completionItem = {
							commitCharactersSupport = false,
							deprecatedSupport = false,
							documentationFormat = { "markdown", "plaintext" },
							preselectSupport = false,
							snippetSupport = false,
						},
						completionItemKind = {
							valueSet = {
								1,
								2,
								3,
								4,
								5,
								6,
								7,
								8,
								9,
								10,
								11,
								12,
								13,
								14,
								15,
								16,
								17,
								18,
								19,
								20,
								21,
								22,
								23,
								24,
								25,
							},
						},
						contextSupport = false,
						dynamicRegistration = false,
					},
					declaration = { linkSupport = true },
					definition = { linkSupport = true },
					documentHighlight = { dynamicRegistration = false },
					documentSymbol = {
						dynamicRegistration = false,
						hierarchicalDocumentSymbolSupport = true,
						symbolKind = {
							valueSet = {
								1,
								2,
								3,
								4,
								5,
								6,
								7,
								8,
								9,
								10,
								11,
								12,
								13,
								14,
								15,
								16,
								17,
								18,
								19,
								20,
								21,
								22,
								23,
								24,
								25,
								26,
							},
						},
					},
					hover = {
						contentFormat = { "markdown", "plaintext" },
						dynamicRegistration = false,
					},
					implementation = { linkSupport = true },
					publishDiagnostics = {
						relatedInformation = true,
						tagSupport = { valueSet = { 1, 2 } },
					},
					references = { dynamicRegistration = false },
					rename = { dynamicRegistration = false, prepareSupport = true },
					semanticTokens = {
						augmentsSyntaxTokens = true,
						dynamicRegistration = false,
						formats = { "relative" },
						multilineTokenSupport = false,
						overlappingTokenSupport = true,
						requests = {
							full = { delta = true },
							range = false,
						},
						serverCancelSupport = false,
						tokenModifiers = {
							"declaration",
							"definition",
							"readonly",
							"static",
							"deprecated",
							"abstract",
							"async",
							"modification",
							"documentation",
							"defaultLibrary",
						},
						tokenTypes = {
							"namespace",
							"type",
							"class",
							"enum",
							"interface",
							"struct",
							"typeParameter",
							"parameter",
							"variable",
							"property",
							"enumMember",
							"event",
							"function",
							"method",
							"macro",
							"keyword",
							"modifier",
							"comment",
							"string",
							"number",
							"regexp",
							"operator",
							"decorator",
						},
					},
					signatureHelp = {
						dynamicRegistration = false,
						signatureInformation = {
							activeParameterSupport = true,
							documentationFormat = { "markdown", "plaintext" },
							parameterInformation = { labelOffsetSupport = true },
						},
					},
					synchronization = {
						didSave = true,
						dynamicRegistration = false,
						willSave = true,
						willSaveWaitUntil = true,
					},
					typeDefinition = { linkSupport = true },
				},
				window = {
					showDocument = { support = true },
					showMessage = {
						messageActionItem = { additionalPropertiesSupport = false },
					},
					workDoneProgress = true,
				},
				workspace = {
					applyEdit = true,
					configuration = true,
					didChangeWatchedFiles = {
						dynamicRegistration = false,
						relativePatternSupport = true,
					},
					semanticTokens = { refreshSupport = true },
					symbol = {
						dynamicRegistration = false,
						hierarchicalWorkspaceSymbolSupport = true,
						symbolKind = {
							valueSet = {
								1,
								2,
								3,
								4,
								5,
								6,
								7,
								8,
								9,
								10,
								11,
								12,
								13,
								14,
								15,
								16,
								17,
								18,
								19,
								20,
								21,
								22,
								23,
								24,
								25,
								26,
							},
						},
					},
					workspaceEdit = { resourceOperations = { "rename", "create", "delete" } },
					workspaceFolders = true,
				},
			},
			cmd = { "/Users/wongjiahau/.local/share/nvim/mason/bin/typescript-language-server", "--stdio" },
			cmd_cwd = "/Users/wongjiahau/repos/editor-idea",
			filetypes = {
				"javascript",
				"javascriptreact",
				"javascript.jsx",
				"typescript",
				"typescriptreact",
				"typescript.tsx",
			},
			flags = {},
			get_language_id = nil,
			handlers = {},
			init_options = {
				hostInfo = "neovim",
				preferences = { importModuleSpecifierPreference = "relative" },
			},
			log_level = 2,
			message_level = 2,
			name = "tsserver",
			on_attach = nil,
			on_exit = nil,
			on_init = nil,
			root_dir = "/Users/wongjiahau/repos/editor-idea",
			settings = vim.empty_dict(),
			single_file_support = true,
			workspace_folders = {
				{
					name = "/Users/wongjiahau/repos/editor-idea",
					uri = "file:///Users/wongjiahau/repos/editor-idea",
				},
			},
			metatable = { __tostring = nil },
		},
		handlers = nil,
		id = 1,
		initialized = true,
		is_stopped = nil,
		messages = {
			messages = {},
			name = "tsserver",
			progress = {
				["d02e3d15-6834-4e04-98cd-e4360a8b60c5"] = {
					done = true,
					title = "Initializing JS/TS language features…",
				},
			},
			status = {},
		},
		name = "tsserver",
		notify = nil,
		offset_encoding = "utf-16",
		request = nil,
		request_sync = nil,
		requests = {},
		rpc = { is_closing = nil, notify = nil, request = nil, terminate = nil },
		server_capabilities = {
			callHierarchyProvider = true,
			codeActionProvider = {
				codeActionKinds = {
					"source.fixAll.ts",
					"source.removeUnused.ts",
					"source.addMissingImports.ts",
					"source.organizeImports.ts",
					"source.removeUnusedImports.ts",
					"source.sortImports.ts",
					"quickfix",
					"refactor",
				},
			},
			completionProvider = {
				resolveProvider = true,
				triggerCharacters = { ".", '"', "'", "/", "@", "<" },
			},
			definitionProvider = true,
			documentFormattingProvider = true,
			documentHighlightProvider = true,
			documentRangeFormattingProvider = true,
			documentSymbolProvider = true,
			executeCommandProvider = {
				commands = {
					"_typescript.applyWorkspaceEdit",
					"_typescript.applyCodeAction",
					"_typescript.applyRefactoring",
					"_typescript.configurePlugin",
					"_typescript.organizeImports",
					"_typescript.applyRenameFile",
					"_typescript.goToSourceDefinition",
				},
			},
			foldingRangeProvider = true,
			hoverProvider = true,
			implementationProvider = true,
			inlayHintProvider = true,
			referencesProvider = true,
			renameProvider = { prepareProvider = true },
			selectionRangeProvider = true,
			semanticTokensProvider = {
				full = true,
				legend = {
					tokenModifiers = { "declaration", "static", "async", "readonly", "defaultLibrary", "local" },
					tokenTypes = {
						"class",
						"enum",
						"interface",
						"namespace",
						"typeParameter",
						"type",
						"parameter",
						"variable",
						"enumMember",
						"property",
						"function",
						"member",
					},
				},
				range = true,
			},
			signatureHelpProvider = {
				retriggerCharacters = { ")" },
				triggerCharacters = { "(", ",", "<" },
			},
			textDocumentSync = {
				change = 2,
				openClose = true,
				save = { includeText = false },
				willSave = false,
				willSaveWaitUntil = false,
			},
			typeDefinitionProvider = true,
			workspace = {
				fileOperations = {
					willRename = {
						filters = {
							{
								pattern = {
									glob = "**/*.{ts,js,jsx,tsx,mjs,mts,cjs,cts}",
									matches = "file",
								},
								scheme = "file",
							},
						},
					},
				},
			},
			workspaceSymbolProvider = true,
		},
		stop = nil,
		supports_method = nil,
		workspace_did_change_configuration = nil,
		workspace_folders = nil,
	},
	{
		_on_attach = nil,
		attached_buffers = { true },
		cancel_request = nil,
		commands = {},
		config = {
			cmd = { "node", "/Users/wongjiahau/.config/nvim/plugged/copilot.vim/dist/agent.js" },
			cmd_cwd = "/Users/wongjiahau",
			flags = {},
			get_language_id = nil,
			handlers = {
				LogMessage = nil,
				PanelSolution = nil,
				PanelSolutionsDone = nil,
				statusNotification = nil,
			},
			name = "copilot",
			on_exit = nil,
			on_init = nil,
			settings = {},
		},
		handlers = nil,
		id = 2,
		initialized = true,
		is_stopped = nil,
		messages = { messages = {}, name = "copilot", progress = {}, status = {} },
		name = "copilot",
		notify = nil,
		offset_encoding = "utf-16",
		request = nil,
		request_sync = nil,
		requests = {},
		rpc = { is_closing = nil, notify = nil, request = nil, terminate = nil },
		server_capabilities = {
			textDocumentSync = { change = 2, openClose = true },
			workspace = {
				workspaceFolders = { changeNotifications = true, supported = true },
			},
		},
		stop = nil,
		supports_method = nil,
	},
}

x = { id = 4, jsonrpc = "2.0", method = "shutdown" }

x = { id = 3, jsonrpc = "2.0", method = "shutdown" }

x = { id = 4, jsonrpc = "2.0" }

x = { id = 3, jsonrpc = "2.0" }

x = { jsonrpc = "2.0", method = "exit" }

x = { jsonrpc = "2.0", method = "exit" }
