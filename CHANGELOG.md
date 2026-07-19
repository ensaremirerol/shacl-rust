# Changelog

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
