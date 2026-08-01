# Changelog

## [0.2.12](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.11...shacl-rust-v0.2.12) (2026-08-01)


### Bug Fixes

* stable source_shape in validation diagnostics (closes R-2 Join gap) ([66d53fc](https://github.com/ensaremirerol/shacl-rust/commit/66d53fce5c39475260f574eb7acc4b93caed44b8))

## [0.2.11](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.10...shacl-rust-v0.2.11) (2026-07-31)


### Features

* **cli:** add `capabilities` subcommand reporting engine name/version/feature flags ([3f5e2d2](https://github.com/ensaremirerol/shacl-rust/commit/3f5e2d26bbf324f3cbef1e9ab1d08d55b40429ef))
* **lint:** add L0013 for sh:ignoredProperties that isn't a well-formed rdf:List ([009ec26](https://github.com/ensaremirerol/shacl-rust/commit/009ec2640242a7b16da7632bc40ad4ce41e480f7))
* named-source collision detection (R-3) and two new lint rules (R-4) ([7a38963](https://github.com/ensaremirerol/shacl-rust/commit/7a38963ae0be715b07e275bf9b8ab32666b39f99))
* shapes decomposition with content-stable IDs (R-1/R-2), fix shacl-mcp --version hang (R-6) ([b1586e3](https://github.com/ensaremirerol/shacl-rust/commit/b1586e3e2a8d837d7a2bd4e73d7746e812389755))

## [0.2.10](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.9...shacl-rust-v0.2.10) (2026-07-28)


### Features

* **mcp:** file-path inputs, shape-file batching, diagnostic summaries ([e1fb3dc](https://github.com/ensaremirerol/shacl-rust/commit/e1fb3dccdb4e1e8eb02c21e2746b0487be890ea2))

## [0.2.9](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.8...shacl-rust-v0.2.9) (2026-07-24)


### Features

* show the traced NodeShape itself in why-trace output ([11d8c46](https://github.com/ensaremirerol/shacl-rust/commit/11d8c46a28e11221dd645290d896e7fe9a16f086))


### Bug Fixes

* build the SPARQL validation store once under contention, not per thread ([2bd0f73](https://github.com/ensaremirerol/shacl-rust/commit/2bd0f73750b4b6f60d6237582edf0b4ab1831ebd))
* default skip_lint in MCP schema; cross-reference datatype+range diagnostics ([f1f430a](https://github.com/ensaremirerol/shacl-rust/commit/f1f430a109be7b2b62191d3c13e8a4f3a1bad190))


### Documentation

* implementation plan for MCP diagnostics tools ([0ed72bc](https://github.com/ensaremirerol/shacl-rust/commit/0ed72bc4a13a9caca1c2d7b9e7f29e4cd078a6a0))

## [0.2.8](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.7...shacl-rust-v0.2.8) (2026-07-23)


### Features

* derive rich diagnostics from validation reports ([a090833](https://github.com/ensaremirerol/shacl-rust/commit/a090833e519b22f3e827e5f5df19e1e2cdcaf4d5))
* diagnostic code registry with explain entries ([b8bf883](https://github.com/ensaremirerol/shacl-rust/commit/b8bf88377151f60ccb380cd6e2476883b26cf25a))
* diagnostic model and ValidationResult accessors ([3810784](https://github.com/ensaremirerol/shacl-rust/commit/38107844f4730f06cc9e1b00687726959524f86c))
* NDJSON renderer for diagnostics ([38a61c0](https://github.com/ensaremirerol/shacl-rust/commit/38a61c0817a2d0aac69e4e563de4da767575c092))
* shape linter with 12 rules ([fc2ec88](https://github.com/ensaremirerol/shacl-rust/commit/fc2ec88b999ba7873e2108c611b2ef9e93fa2ac9))
* shape_target_nodes helper for browsing all resolved focus nodes ([32029a0](https://github.com/ensaremirerol/shacl-rust/commit/32029a062f5d40b77426c1fd4f1a1ef301d414d8))
* terminal renderer for diagnostics ([0c06873](https://github.com/ensaremirerol/shacl-rust/commit/0c06873c65810e04d247abcbaf27749ed4b0366d))
* **wasm:** expose rich diagnostics; add Diagnostics output mode to web demo ([72efbbc](https://github.com/ensaremirerol/shacl-rust/commit/72efbbc373f473944b7cc9466e1b107027e7cdfe))
* **web:** diagnostics-first layout — summary banner, diagnostics list, shapes panel, raw report disclosure ([1efe612](https://github.com/ensaremirerol/shacl-rust/commit/1efe61211e9cb7bf414c7565e8eeca2c6e861e33))
* **web:** pure rendering module for the diagnostics-first UI ([0610468](https://github.com/ensaremirerol/shacl-rust/commit/06104681f0c18aee1486b82ed877dfed44b4ac5b))
* **web:** wire diagnostics-first UI to new wasm JSON exports ([5d5768f](https://github.com/ensaremirerol/shacl-rust/commit/5d5768f0663983956b676e4e41d6fc1fe6460fab))
* why subcommand explaining conformance per focus node ([9f326aa](https://github.com/ensaremirerol/shacl-rust/commit/9f326aa56a13e8fee2b66c41abb106b892c31024))


### Bug Fixes

* caret runs underline the full highlight span ([6677456](https://github.com/ensaremirerol/shacl-rust/commit/667745659bf572520769c2975c52de5d11c25772))
* disambiguate same-component constraints when matching why-trace results ([282b2f8](https://github.com/ensaremirerol/shacl-rust/commit/282b2f8451f0ef81ef17eb9b6b0261631ba7ceb6))
* L0005 permits property-pair constraints on node shapes ([0e47570](https://github.com/ensaremirerol/shacl-rust/commit/0e47570f4ebd437a5f8fc259b55f3d5786efde9f))
* robust first-sentence help and exact-local-name constraint highlighting ([7fbd5a2](https://github.com/ensaremirerol/shacl-rust/commit/7fbd5a2cff4a19ddcddba7d63c0b2a64de80a963))
* why-panel/shapes-panel blank-node source_shape correlation + wasm fmt ([3842e84](https://github.com/ensaremirerol/shacl-rust/commit/3842e842bce9483b525a7c3db4a359dc39a87889))


### Documentation

* correct V0021 explain entry to engine-honest phrasing ([d8e7b51](https://github.com/ensaremirerol/shacl-rust/commit/d8e7b515320369c5a33f5361487dec0caa699be7))
* design spec for rustc-style diagnostics ([efcf448](https://github.com/ensaremirerol/shacl-rust/commit/efcf448fd8474b2fd4a3a1fad47b51898d4ee60a))
* design spec for web diagnostics UX redesign + deferred MCP integration ([c04e1dc](https://github.com/ensaremirerol/shacl-rust/commit/c04e1dcd5fad90cf7f96225dfc48086efd474136))
* diagnostics usage in READMEs ([d104a7e](https://github.com/ensaremirerol/shacl-rust/commit/d104a7e96495dd13bd4a097d1362a3ea8f13fe1d))
* implementation plan for rustc-style diagnostics ([5ce2d81](https://github.com/ensaremirerol/shacl-rust/commit/5ce2d81d8707a2f652595414ca05ce3a2ae1764b))
* implementation plan for web diagnostics UX redesign ([f36f759](https://github.com/ensaremirerol/shacl-rust/commit/f36f75958077d19f7b27dbc67760bdeba7173235))

## [0.2.7](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.6...shacl-rust-v0.2.7) (2026-07-20)


### Bug Fixes

* pre-bind SPARQL variables outside the SELECT projection ([d2f59d3](https://github.com/ensaremirerol/shacl-rust/commit/d2f59d341c2eda9a0086b577d6efe3452d1935be))
* serialize full sh:resultPath structures in RDF reports ([94bfda0](https://github.com/ensaremirerol/shacl-rust/commit/94bfda0fd45eb20b691d1038c71e9f6328d7c757))
* spec-correct result cardinality and value-node coverage in core constraints ([5fc98d7](https://github.com/ensaremirerol/shacl-rust/commit/5fc98d7b3ad1ebe953fbabdda0a52190df2a7053))

## [0.2.6](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.5...shacl-rust-v0.2.6) (2026-07-20)


### Features

* evaluate sh:SPARQLTarget ([8fb3f11](https://github.com/ensaremirerol/shacl-rust/commit/8fb3f11ff4720eca1d11a188cd2582885f731a86))


### Bug Fixes

* real SHACL pre-binding semantics for sh:sparql constraints ([2ed271f](https://github.com/ensaremirerol/shacl-rust/commit/2ed271feda1bbe9dc8110e48e4517203f598ccab))

## [0.2.5](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.4...shacl-rust-v0.2.5) (2026-07-20)


### Performance Improvements

* CSR index layout and FxHash for interning and hot sets ([2d6e780](https://github.com/ensaremirerol/shacl-rust/commit/2d6e78003873e0b610e30517008515ede545d399))


### Miscellaneous

* assert u32 term-id capacity in IndexedGraph interner ([d9199b0](https://github.com/ensaremirerol/shacl-rust/commit/d9199b040b876c185538439bd40fa26ae83c7a6b))

## [0.2.4](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.3...shacl-rust-v0.2.4) (2026-07-20)


### Features

* Python bindings published to PyPI as shacl-rust ([c0277b8](https://github.com/ensaremirerol/shacl-rust/commit/c0277b81035f244c8dad5c5637e8bc2dcb633d44))
* stream graph loading from readers everywhere ([a4fcbf0](https://github.com/ensaremirerol/shacl-rust/commit/a4fcbf0a3dcb6dd13c4e0bdd30b620d6ee7e1e51))


### Bug Fixes

* honor sh:ignoredProperties lists on closed shapes ([e685659](https://github.com/ensaremirerol/shacl-rust/commit/e6856596ddb9d5c8f1825c5cea25259f1e46807a))

## [0.2.3](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.2...shacl-rust-v0.2.3) (2026-07-20)


### Features

* experimental interned data-graph index backend ([7b14624](https://github.com/ensaremirerol/shacl-rust/commit/7b146245ddfd9ea9ab4a15635815464136b0545b))


### Performance Improvements

* stream parsed quads directly into the graph ([cfd5050](https://github.com/ensaremirerol/shacl-rust/commit/cfd5050194aa0003087e026e8f63d638f1cc0f65))

## [0.2.2](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.1...shacl-rust-v0.2.2) (2026-07-19)


### Miscellaneous

* release main ([6c3f128](https://github.com/ensaremirerol/shacl-rust/commit/6c3f1284ecdf32f864da71766934041125ed8a22))

## [0.2.1](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.2.0...shacl-rust-v0.2.1) (2026-07-19)


### Miscellaneous

* align Cargo.lock with crate versions ([5c2737b](https://github.com/ensaremirerol/shacl-rust/commit/5c2737b999a2b3465fd2aa2d080110eb793231c5))
* enable cargo-workspace plugin for release-please ([ece2593](https://github.com/ensaremirerol/shacl-rust/commit/ece259349c99e9075917cf41de20c926f17ad199))
* scope root package commits with exclude-paths in release-please ([be147ef](https://github.com/ensaremirerol/shacl-rust/commit/be147ef99a6a7f6d481340a86be176218d1ec2c7))

## [0.2.0](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.7...shacl-rust-v0.2.0) (2026-07-19)


### ⚠ BREAKING CHANGES

* remove unused inject_values_bindings helper

### refactor

* remove unused inject_values_bindings helper ([bf14bfc](https://github.com/ensaremirerol/shacl-rust/commit/bf14bfc5fa1e2eeebf90ff46494f02936a7e628e))


### Bug Fixes

* sh:class matches instances of rdfs:subClassOf subclasses ([e87313f](https://github.com/ensaremirerol/shacl-rust/commit/e87313f604768cb7460bd965e1f9890ba72121f6))


### Performance Improvements

* build SPARQL store lazily in one transaction, hoist constant bindings ([95eb231](https://github.com/ensaremirerol/shacl-rust/commit/95eb2315cf8dc145f91fb6f67d63ae9bdcdd8a41))
* compile sh:pattern regex once at parse time instead of per focus node ([17100ee](https://github.com/ensaremirerol/shacl-rust/commit/17100ee9cf88c2785658b587795ddc7f036baf62))
* expand property paths via callback to avoid per-step allocations ([f52562f](https://github.com/ensaremirerol/shacl-rust/commit/f52562f17bbe3e6dbe3ef415202070710d285442))
* parse sh:sparql queries once and pre-bind terms via variable substitution ([7073b09](https://github.com/ensaremirerol/shacl-rust/commit/7073b09edb37f73ee8a186a3f85da911f021e6fe))
* pre-parse comparison literals and build violation strings lazily ([1777ec8](https://github.com/ensaremirerol/shacl-rust/commit/1777ec8252fd2d34db88010b701f45dde59d150e))
* remove redundant O(n^2) frontier scans in subclass/subproperty traversal ([1c1001f](https://github.com/ensaremirerol/shacl-rust/commit/1c1001fc6769d55f23ca4d494c2489264940f53b))
* use HashSet membership test in sh:in constraint ([4fac93e](https://github.com/ensaremirerol/shacl-rust/commit/4fac93eea85867525283b79558e1917c6bee0d4a))


### Documentation

* mark performance plan as completed ([e177304](https://github.com/ensaremirerol/shacl-rust/commit/e177304dd1f5cc492cfedfcab52fc5d5c7cad778))
* record follow-up bench results and skipped-optimization rationale ([dce40d7](https://github.com/ensaremirerol/shacl-rust/commit/dce40d7b9472cdc5c4619271acce8ee9ce069e0d))

## [0.1.7](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.6...shacl-rust-v0.1.7) (2026-07-19)


### Bug Fixes

* **ci:** make publish-crates fail loudly and tolerate lockfile drift ([dc4ed9f](https://github.com/ensaremirerol/shacl-rust/commit/dc4ed9f8d1565e90bbb147b95cfb2519a1126b7d))
* stop nested-SELECT detection from false-flagging plain ASK queries ([910e1b6](https://github.com/ensaremirerol/shacl-rust/commit/910e1b6f2ad05f3246f44d6944480f15378f8105))

## [0.1.6](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.5...shacl-rust-v0.1.6) (2026-07-19)


### Bug Fixes

* avoid duplicate logger init and accept turtle as output-format alias ([ff37d6a](https://github.com/ensaremirerol/shacl-rust/commit/ff37d6a2b37bc4dbf7b44c75c1f0d0fab86bb73c))
* correctly inject SPARQL VALUES bindings for WHERE-less ASK/SELECT queries ([50cd557](https://github.com/ensaremirerol/shacl-rust/commit/50cd5573e7e4900a891bebaacb3d4b00bd967153))
* disable oxigraph default features in shacl-cli and shacl-mcp ([9c8ef07](https://github.com/ensaremirerol/shacl-rust/commit/9c8ef07b31f7ab56a6e666b526a6b67c62151e65))
* link sh:ValidationReport to results via sh:result instead of sh:detail ([127ad3a](https://github.com/ensaremirerol/shacl-rust/commit/127ad3a35494c5d0dc3bc6d2a9e25d25ee5b905d))


### Performance Improvements

* resolve property paths via indexed graph lookups instead of full scans ([36c3f9d](https://github.com/ensaremirerol/shacl-rust/commit/36c3f9de32d1ec3980d9d8ea170fe00548e2217c))


### Miscellaneous

* account for shacl-cli/shacl-mcp's already-bumped 0.1.5 in release-please manifest ([2cd2d3d](https://github.com/ensaremirerol/shacl-rust/commit/2cd2d3d701f07fb8cfdc3c3f68e4bb0b2e1e9701))

## [0.1.5](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.4...shacl-rust-v0.1.5) (2026-04-06)


### Bug Fixes

* pattern contraint was only checking literals ([a67a3d5](https://github.com/ensaremirerol/shacl-rust/commit/a67a3d524f1c7193aaa626247c2d38217694a043))

## [0.1.4](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.3...shacl-rust-v0.1.4) (2026-02-25)


### Bug Fixes

* ci ([745424b](https://github.com/ensaremirerol/shacl-rust/commit/745424b55c97c8b934736629950aad56612fa250))
* ci ([219c03d](https://github.com/ensaremirerol/shacl-rust/commit/219c03de7042c3714cf0746bc1adf9cdce866fc2))
* ci ([03eae6d](https://github.com/ensaremirerol/shacl-rust/commit/03eae6d0c68dbbda0577a7485841eec3a3c67e84))
* ci ([12bbc72](https://github.com/ensaremirerol/shacl-rust/commit/12bbc7215ca6c8e59b33c09008d00a2f1108b04d))
* ci ([15e8aa0](https://github.com/ensaremirerol/shacl-rust/commit/15e8aa09947558909aa33116c3a3d4479513480c))
* now report is not directly edited by validators ([e055784](https://github.com/ensaremirerol/shacl-rust/commit/e055784f61277b390b1fe8d7dbeebc79225cda76))
* now report text includes source constraint componenet ([195c7b6](https://github.com/ensaremirerol/shacl-rust/commit/195c7b6d640201f1534e059e444083e5a89ee6aa))
* wasm ([f43e64f](https://github.com/ensaremirerol/shacl-rust/commit/f43e64f22c7999d9e3b75bcfca07ecb100a3134c))


### Miscellaneous

* release main ([b5ae5f6](https://github.com/ensaremirerol/shacl-rust/commit/b5ae5f651a9b4c34a4654034f11bda70c37edfbe))
* release main ([52547fd](https://github.com/ensaremirerol/shacl-rust/commit/52547fd5d1f2697aa2f4e62dbf1f88b99f372711))
* release main ([deae4e3](https://github.com/ensaremirerol/shacl-rust/commit/deae4e3dc582085f9573181d319d2a9295d73351))
* release main ([9fc5303](https://github.com/ensaremirerol/shacl-rust/commit/9fc5303f3e4927dc56fc86b058bb9f34c20464aa))
* sync dev branch after merge ([06be7ae](https://github.com/ensaremirerol/shacl-rust/commit/06be7aedfa382c96932ab6083910e9408e8c6b6f))

## [0.1.3](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.2...shacl-rust-v0.1.3) (2026-02-22)


### Features

* **ci:** configure release-please to create PRs from dev to main ([ab7c1d4](https://github.com/ensaremirerol/shacl-rust/commit/ab7c1d45f8b7cafecaa73dafb2d9583029d59ebd))


### Bug Fixes

* **ci:** release-please creates PR from dev to main ([3b8433f](https://github.com/ensaremirerol/shacl-rust/commit/3b8433ff7dd8684da91ac40cfc2e3e00ab065085))
* **ci:** simplify publish conditions to work with any release ([91db136](https://github.com/ensaremirerol/shacl-rust/commit/91db136bc7ba4ada629a87e74fea510e5a2fb2c5))

## [0.1.2](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.1...shacl-rust-v0.1.2) (2026-02-22)


### Bug Fixes

* **ci:** correct release-please output keys for root path publishing ([212b780](https://github.com/ensaremirerol/shacl-rust/commit/212b780e5f8fe2cce4e68a4ca804e7f7b600e924))


### Miscellaneous

* fmt ([c10eeed](https://github.com/ensaremirerol/shacl-rust/commit/c10eeedb992a72f2245d60ec3bc9f9f0285d966b))
* initial release ([a24a9f1](https://github.com/ensaremirerol/shacl-rust/commit/a24a9f131eab908404c703dd2fa6740900c376ea))
* release main ([52b80f5](https://github.com/ensaremirerol/shacl-rust/commit/52b80f5b93b97856385d33415cf26bcf40d46ef4))
* release main ([ecb32d9](https://github.com/ensaremirerol/shacl-rust/commit/ecb32d9e33baa039469f983164db436ff690719f))

## [0.1.1](https://github.com/ensaremirerol/shacl-rust/compare/shacl-rust-v0.1.0...shacl-rust-v0.1.1) (2026-02-22)


### Bug Fixes

* **ci:** correct release-please output keys for root path publishing ([212b780](https://github.com/ensaremirerol/shacl-rust/commit/212b780e5f8fe2cce4e68a4ca804e7f7b600e924))


### Miscellaneous

* fmt ([c10eeed](https://github.com/ensaremirerol/shacl-rust/commit/c10eeedb992a72f2245d60ec3bc9f9f0285d966b))
* initial release ([a24a9f1](https://github.com/ensaremirerol/shacl-rust/commit/a24a9f131eab908404c703dd2fa6740900c376ea))
