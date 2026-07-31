# Changelog

## [0.2.11](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.10...shacl-mcp-v0.2.11) (2026-07-31)


### Features

* **lint:** add L0013 for sh:ignoredProperties that isn't a well-formed rdf:List ([009ec26](https://github.com/ensaremirerol/shacl-rust/commit/009ec2640242a7b16da7632bc40ad4ce41e480f7))
* named-source collision detection (R-3) and two new lint rules (R-4) ([7a38963](https://github.com/ensaremirerol/shacl-rust/commit/7a38963ae0be715b07e275bf9b8ab32666b39f99))
* shapes decomposition with content-stable IDs (R-1/R-2), fix shacl-mcp --version hang (R-6) ([b1586e3](https://github.com/ensaremirerol/shacl-rust/commit/b1586e3e2a8d837d7a2bd4e73d7746e812389755))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.10 to 0.2.11

## [0.2.10](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.9...shacl-mcp-v0.2.10) (2026-07-28)


### Features

* **mcp:** file-path inputs, shape-file batching, diagnostic summaries ([e1fb3dc](https://github.com/ensaremirerol/shacl-rust/commit/e1fb3dccdb4e1e8eb02c21e2746b0487be890ea2))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.9 to 0.2.10

## [0.2.9](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.8...shacl-mcp-v0.2.9) (2026-07-24)


### Features

* **mcp:** add validate_diagnostics/lint_shacl_shapes/explain_diagnostic_code/why_conformance tools ([ddf440f](https://github.com/ensaremirerol/shacl-rust/commit/ddf440f4b7c5ae93e3614fc0fff8cf8ec68d6b33))


### Bug Fixes

* default skip_lint in MCP schema; cross-reference datatype+range diagnostics ([f1f430a](https://github.com/ensaremirerol/shacl-rust/commit/f1f430a109be7b2b62191d3c13e8a4f3a1bad190))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.8 to 0.2.9

## [0.2.8](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.7...shacl-mcp-v0.2.8) (2026-07-23)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.7 to 0.2.8

## [0.2.7](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.6...shacl-mcp-v0.2.7) (2026-07-20)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.6 to 0.2.7

## [0.2.6](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.5...shacl-mcp-v0.2.6) (2026-07-20)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.5 to 0.2.6

## [0.2.5](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.4...shacl-mcp-v0.2.5) (2026-07-20)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.4 to 0.2.5

## [0.2.4](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.3...shacl-mcp-v0.2.4) (2026-07-20)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.3 to 0.2.4

## [0.2.3](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.2...shacl-mcp-v0.2.3) (2026-07-20)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.2 to 0.2.3

## [0.2.2](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.2.0...shacl-mcp-v0.2.2) (2026-07-19)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.1 to 0.2.2

## [0.2.0](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.7...shacl-mcp-v0.2.0) (2026-07-19)


### ⚠ BREAKING CHANGES

* **shacl-mcp:** sync version with shacl-rust 0.2

### Features

* **shacl-mcp:** sync version with shacl-rust 0.2 ([e7de721](https://github.com/ensaremirerol/shacl-rust/commit/e7de72147fe0519aac4b4a5da8a4cfea2e13e236))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * shacl-rust bumped from 0.2.0 to 0.2.1

## [0.1.7](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.6...shacl-mcp-v0.1.7) (2026-07-19)


### Miscellaneous

* release main ([f4c7ccb](https://github.com/ensaremirerol/shacl-rust/commit/f4c7ccb125e6766bf5d7ca0c191cce5b229bd674))

## [0.1.6](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.5...shacl-mcp-v0.1.6) (2026-07-19)


### Bug Fixes

* disable oxigraph default features in shacl-cli and shacl-mcp ([9c8ef07](https://github.com/ensaremirerol/shacl-rust/commit/9c8ef07b31f7ab56a6e666b526a6b67c62151e65))
* now report is not directly edited by validators ([e055784](https://github.com/ensaremirerol/shacl-rust/commit/e055784f61277b390b1fe8d7dbeebc79225cda76))


### Miscellaneous

* fmt ([c10eeed](https://github.com/ensaremirerol/shacl-rust/commit/c10eeedb992a72f2245d60ec3bc9f9f0285d966b))
* initial release ([a24a9f1](https://github.com/ensaremirerol/shacl-rust/commit/a24a9f131eab908404c703dd2fa6740900c376ea))
* release main ([fc5eaa5](https://github.com/ensaremirerol/shacl-rust/commit/fc5eaa5e4eabe8738b8c2cce0162c9742293f723))
* release main ([10629c5](https://github.com/ensaremirerol/shacl-rust/commit/10629c5bbe905183d3676eb1607ced6f8bccf1fd))
* release main ([1b0e717](https://github.com/ensaremirerol/shacl-rust/commit/1b0e717e7714e1908d6d28e5ae187e7cfd8ca1d9))
* release main ([b5ae5f6](https://github.com/ensaremirerol/shacl-rust/commit/b5ae5f651a9b4c34a4654034f11bda70c37edfbe))
* release main ([52547fd](https://github.com/ensaremirerol/shacl-rust/commit/52547fd5d1f2697aa2f4e62dbf1f88b99f372711))
* release main ([deae4e3](https://github.com/ensaremirerol/shacl-rust/commit/deae4e3dc582085f9573181d319d2a9295d73351))
* release main ([9fc5303](https://github.com/ensaremirerol/shacl-rust/commit/9fc5303f3e4927dc56fc86b058bb9f34c20464aa))
* release main ([43e02a2](https://github.com/ensaremirerol/shacl-rust/commit/43e02a2183ba85e9d8e19fd90787378348acdec2))
* release main ([c4bdac9](https://github.com/ensaremirerol/shacl-rust/commit/c4bdac90c800eadace695b6d05fc3e4e4941f2d2))
* release main ([19e039d](https://github.com/ensaremirerol/shacl-rust/commit/19e039d53d10ed558293a33b36b04d17ffe2f876))
* release main ([8a38f99](https://github.com/ensaremirerol/shacl-rust/commit/8a38f998da65ae6c030e7b79639124ed7830cc90))
* release main ([52b80f5](https://github.com/ensaremirerol/shacl-rust/commit/52b80f5b93b97856385d33415cf26bcf40d46ef4))
* release main ([ecb32d9](https://github.com/ensaremirerol/shacl-rust/commit/ecb32d9e33baa039469f983164db436ff690719f))

## [0.1.4](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.3...shacl-mcp-v0.1.4) (2026-02-25)


### Bug Fixes

* now report is not directly edited by validators ([e055784](https://github.com/ensaremirerol/shacl-rust/commit/e055784f61277b390b1fe8d7dbeebc79225cda76))


### Miscellaneous

* release main ([b5ae5f6](https://github.com/ensaremirerol/shacl-rust/commit/b5ae5f651a9b4c34a4654034f11bda70c37edfbe))
* release main ([52547fd](https://github.com/ensaremirerol/shacl-rust/commit/52547fd5d1f2697aa2f4e62dbf1f88b99f372711))

## [0.1.3](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.2...shacl-mcp-v0.1.3) (2026-02-22)


### Miscellaneous

* release main ([43e02a2](https://github.com/ensaremirerol/shacl-rust/commit/43e02a2183ba85e9d8e19fd90787378348acdec2))
* release main ([c4bdac9](https://github.com/ensaremirerol/shacl-rust/commit/c4bdac90c800eadace695b6d05fc3e4e4941f2d2))

## [0.1.2](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.1...shacl-mcp-v0.1.2) (2026-02-22)


### Miscellaneous

* fmt ([c10eeed](https://github.com/ensaremirerol/shacl-rust/commit/c10eeedb992a72f2245d60ec3bc9f9f0285d966b))
* initial release ([a24a9f1](https://github.com/ensaremirerol/shacl-rust/commit/a24a9f131eab908404c703dd2fa6740900c376ea))
* release main ([52b80f5](https://github.com/ensaremirerol/shacl-rust/commit/52b80f5b93b97856385d33415cf26bcf40d46ef4))
* release main ([ecb32d9](https://github.com/ensaremirerol/shacl-rust/commit/ecb32d9e33baa039469f983164db436ff690719f))

## [0.1.1](https://github.com/ensaremirerol/shacl-rust/compare/shacl-mcp-v0.1.0...shacl-mcp-v0.1.1) (2026-02-22)


### Miscellaneous

* fmt ([c10eeed](https://github.com/ensaremirerol/shacl-rust/commit/c10eeedb992a72f2245d60ec3bc9f9f0285d966b))
* initial release ([a24a9f1](https://github.com/ensaremirerol/shacl-rust/commit/a24a9f131eab908404c703dd2fa6740900c376ea))
