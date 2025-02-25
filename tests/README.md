# 🧪 Siren Testing Guide

This directory contains tests for the Siren project. The tests are organized as follows:

## 🧩 Test Structure

- **Unit Tests**: Located in the same file as the code they test (within the `src` directory)
- **Integration Tests**: Located in this directory, testing how components work together

## 📁 Test Files

- `mod.rs`: The main test module that makes all tests available to the test runner
- `language_tests.rs`: Tests for the Language enum and related functionality
- `registry_tests.rs`: Tests for the tool registry system
- `executor_tests.rs`: Tests for the parallel tool execution functionality

## 🛠️ Mock Implementations

Each test file that needs mock objects contains a local `test_mocks` module with implementations 
specific to the tests in that file. This approach allows each test file to have specialized
mock implementations as needed.

## 🚀 Running Tests

To run all tests:

```bash
cargo test
```

To run specific tests:

```bash
cargo test language_tests  # Run only language tests
cargo test test_registry   # Run tests with "test_registry" in the name
```

## 📈 Test Coverage

The tests focus on core functionality:

1. **Language Detection**: Tests for correctly identifying languages from file extensions
2. **Tool Registry**: Tests for registering, retrieving, and filtering tools
3. **Tool Execution**: Tests for running tools in parallel and handling results

## 🧠 Testing Philosophy

1. **Mock Dependencies**: Use mock implementations to avoid external dependencies
2. **Test Small Units**: Focus on testing small units of functionality
3. **Integration Tests**: Ensure components work together correctly
4. **Parameterized Tests**: Use rstest for parameterized tests where appropriate 