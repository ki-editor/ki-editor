/** @type {import('jest').Config} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  roots: ["<rootDir>/src"],
  testMatch: ["**/test/**/*.test.ts"],
  transform: {
    "^.+\\.tsx?$": [
      "ts-jest",
      {
        tsconfig: "src/tsconfig.json",
      },
    ],
  },
  setupFilesAfterEnv: ["<rootDir>/src/test/setup.ts"],
  collectCoverage: true,
  coverageDirectory: "coverage",
  coverageReporters: ["text", "lcov"],
  coverageThreshold: {
    global: {
      branches: 90,
      functions: 90,
      lines: 90,
      statements: 90,
    },
  },
  moduleNameMapper: {
    "^vscode$": "<rootDir>/src/test/vscode.mock.ts",
  },
  verbose: true,
  testTimeout: 10000, // 10 seconds for integration/e2e tests
  maxWorkers: "50%", // Limit parallel test execution
};
