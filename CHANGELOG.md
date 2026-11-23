# Changelog

## [0.9.0](https://github.com/sebastian-software/ferrigura/compare/v0.8.2...v0.9.0) (2026-11-23)


### Features

* add a design mode for flowcharts ([#210](https://github.com/sebastian-software/ferrigura/issues/210)) ([b07ee57](https://github.com/sebastian-software/ferrigura/commit/b07ee57355d70b138d9f681a3e58f2cda761a0b2))
* **cli:** fetch pinned catalog fonts ([#185](https://github.com/sebastian-software/ferrigura/issues/185)) ([362faf2](https://github.com/sebastian-software/ferrigura/commit/362faf2464a608809b6154c9efe09c3a338e7c4d))
* **cli:** support protected font downloads ([#208](https://github.com/sebastian-software/ferrigura/issues/208)) ([2127f8e](https://github.com/sebastian-software/ferrigura/commit/2127f8e01e8301a9b87a8ea85733ada19848deeb))
* **elk:** port the network simplex node placement and use it in the design mode ([#222](https://github.com/sebastian-software/ferrigura/issues/222)) ([bfa668d](https://github.com/sebastian-software/ferrigura/commit/bfa668dd2a740fb460e975c48851e1f2e26de834))
* expand the static font catalog ([#186](https://github.com/sebastian-software/ferrigura/issues/186)) ([c49f2ab](https://github.com/sebastian-software/ferrigura/commit/c49f2aba3023b0b9cce8565251788d088f52f7c1))
* **node:** resolve pinned catalog fonts ([#189](https://github.com/sebastian-software/ferrigura/issues/189)) ([0cbe74d](https://github.com/sebastian-software/ferrigura/commit/0cbe74df8b42a3c9f61fa0d31c8b88a139615d5a))
* **node:** support protected font downloads ([#209](https://github.com/sebastian-software/ferrigura/issues/209)) ([a3dddb1](https://github.com/sebastian-software/ferrigura/commit/a3dddb12b82404027aa35ee89f59cf601e235a6e))
* place the nodes of the design mode on axes and bundle the edges of a side ([#227](https://github.com/sebastian-software/ferrigura/issues/227)) ([7dbc32b](https://github.com/sebastian-software/ferrigura/commit/7dbc32b8578e8aea1d27f3da63243b2f69ca3c09))


### Bug Fixes

* accept binary content type for font downloads ([#179](https://github.com/sebastian-software/ferrigura/issues/179)) ([7af2b41](https://github.com/sebastian-software/ferrigura/commit/7af2b41551700455db50ee6d054cb68da12c907d))
* **deps:** update rust crates ([#101](https://github.com/sebastian-software/ferrigura/issues/101)) ([85eed85](https://github.com/sebastian-software/ferrigura/commit/85eed855f3b3f2c910eaef92d7f7bc327122d812))
* **port:** do not walk into dependency folders of the reference checkout ([#231](https://github.com/sebastian-software/ferrigura/issues/231)) ([78c6915](https://github.com/sebastian-software/ferrigura/commit/78c6915740f7bf49dde3e33cfe0203506f2b9570))


### Performance Improvements

* batch cubic length samples ([#183](https://github.com/sebastian-software/ferrigura/issues/183)) ([3a092cb](https://github.com/sebastian-software/ferrigura/commit/3a092cb9ea8096be28d8eb1e78fdf86ab6448d94))
* **elk:** key property holders by a dense option index ([#212](https://github.com/sebastian-software/ferrigura/issues/212)) ([4c4438a](https://github.com/sebastian-software/ferrigura/commit/4c4438a010358940fd244b0ae912ba6214ff0ffb))
* **elk:** read layout options without intermediate string pairs ([#229](https://github.com/sebastian-software/ferrigura/issues/229)) ([cd6a6d9](https://github.com/sebastian-software/ferrigura/commit/cd6a6d9b2bd0ab039103e7122fbac43f790fa6ed))
* **elk:** skip unused ordering log formatting ([#198](https://github.com/sebastian-software/ferrigura/issues/198)) ([5ffd2ab](https://github.com/sebastian-software/ferrigura/commit/5ffd2ab078e5531b0ec2078db784f6243e7910f1))
* **ferrigura:** resolve directive configurations with less work ([#214](https://github.com/sebastian-software/ferrigura/issues/214)) ([343532c](https://github.com/sebastian-software/ferrigura/commit/343532c3d30d501cabb0627cbc6e87bb0eccf252))
* **jsmath:** add a double-double fast path for sin and cos ([#213](https://github.com/sebastian-software/ferrigura/issues/213)) ([ab31e97](https://github.com/sebastian-software/ferrigura/commit/ab31e9731f6e9e813b14569e53eafa357a56e815))
* **model:** reduce number formatting allocations ([#182](https://github.com/sebastian-software/ferrigura/issues/182)) ([2e22739](https://github.com/sebastian-software/ferrigura/commit/2e227391209f39f36ef552aeae76278c5f333d27))
* **render:** avoid line-jump segment allocations ([#190](https://github.com/sebastian-software/ferrigura/issues/190)) ([4b60958](https://github.com/sebastian-software/ferrigura/commit/4b609583a696049596e5efda0fcb4a9aef67ce45))
* **render:** avoid resolving direction for every node ([#192](https://github.com/sebastian-software/ferrigura/issues/192)) ([ae0c4a9](https://github.com/sebastian-software/ferrigura/commit/ae0c4a9280c94d2f0763414ae5aeb24f1221b20e))
* **render:** borrow measured outlines during layout ([#201](https://github.com/sebastian-software/ferrigura/issues/201)) ([f3e61bf](https://github.com/sebastian-software/ferrigura/commit/f3e61bf03495cd727472bde00588097a7e17af2a))
* **render:** consume finished ELK arena nodes ([#202](https://github.com/sebastian-software/ferrigura/issues/202)) ([9ec9bbb](https://github.com/sebastian-software/ferrigura/commit/9ec9bbb4ddc99e90579469e25a0abbdd48a3253f))
* **render:** format coordinates without a heap string ([#223](https://github.com/sebastian-software/ferrigura/issues/223)) ([c8db1cb](https://github.com/sebastian-software/ferrigura/commit/c8db1cbc52c42daa7f88373d53fe19d5d471e737))
* **render:** parse an edge's path once for its length and bounds ([#224](https://github.com/sebastian-software/ferrigura/issues/224)) ([6e12a57](https://github.com/sebastian-software/ferrigura/commit/6e12a57cbfcd3b24e9e28062c858edde2bfb73aa))
* **render:** reduce flowchart SVG assembly allocations ([#207](https://github.com/sebastian-software/ferrigura/issues/207)) ([3ba6b01](https://github.com/sebastian-software/ferrigura/commit/3ba6b01fd674346ad2e9182921fb04f2d3330752))
* **render:** reuse ELK input when rendering ([#206](https://github.com/sebastian-software/ferrigura/issues/206)) ([eb1dbca](https://github.com/sebastian-software/ferrigura/commit/eb1dbca6c13e4967d692965d7a4e70a3655d60d1))
* **render:** reuse flowchart stylesheet buffer ([#195](https://github.com/sebastian-software/ferrigura/issues/195)) ([3a598b3](https://github.com/sebastian-software/ferrigura/commit/3a598b32e053d7d63de031b9fccd958828979060))
* **render:** write SVG text into its buffer instead of through format! ([#218](https://github.com/sebastian-software/ferrigura/issues/218)) ([caacd52](https://github.com/sebastian-software/ferrigura/commit/caacd52fa4357dcb371035c4b2bb475baaeae06a))
* **render:** write the flowchart style sheet into one buffer ([#226](https://github.com/sebastian-software/ferrigura/issues/226)) ([b0e43a4](https://github.com/sebastian-software/ferrigura/commit/b0e43a4f7fbc1b6bad633cf08db506b2baa343e8))
* **swimlane:** dedup candidate paths by integer keys ([#211](https://github.com/sebastian-software/ferrigura/issues/211)) ([1d09e51](https://github.com/sebastian-software/ferrigura/commit/1d09e514d8bffb374ef78bf61d0c1efc71d1a5a4))
* **swimlane:** hash the candidate-path dedup sets with a local hasher ([#217](https://github.com/sebastian-software/ferrigura/issues/217)) ([ee3aa20](https://github.com/sebastian-software/ferrigura/commit/ee3aa205db69250d19814859c69a2d5128747956))
* **text:** reuse registered CFF font bytes ([#196](https://github.com/sebastian-software/ferrigura/issues/196)) ([e2e9666](https://github.com/sebastian-software/ferrigura/commit/e2e9666639a8e0ba36b94d223eac082722c710d7))
* **text:** reuse shape plans per face ([#216](https://github.com/sebastian-software/ferrigura/issues/216)) ([d3fb6e2](https://github.com/sebastian-software/ferrigura/commit/d3fb6e2d9942f0fcc1d2186389d662e4a36800be))
* use mimalloc in the CLI and the Node.js addon ([#215](https://github.com/sebastian-software/ferrigura/issues/215)) ([c9e655c](https://github.com/sebastian-software/ferrigura/commit/c9e655c8f12d219362c853dc027b390a2174b0eb))

## [0.8.2](https://github.com/sebastian-software/ferrigura/compare/v0.8.1...v0.8.2) (2026-11-21)


### Bug Fixes

* **deps:** update URL parsing to 2.5.9 ([#164](https://github.com/sebastian-software/ferrigura/issues/164)) ([be8a0a7](https://github.com/sebastian-software/ferrigura/commit/be8a0a7fe4000c49f001ad8e4001c889f69c788d))


### Performance Improvements

* batch quadratic length samples ([#174](https://github.com/sebastian-software/ferrigura/issues/174)) ([1c15781](https://github.com/sebastian-software/ferrigura/commit/1c15781bbc4a8e2f86ec041835fa23fe25cf468d))
* compile fCoSE constraints before simulation ([#145](https://github.com/sebastian-software/ferrigura/issues/145)) ([e985573](https://github.com/sebastian-software/ferrigura/commit/e9855738242f088f225efa00f51b85e42d321c93))
* **cose:** reuse graph traversal during layout ([#153](https://github.com/sebastian-software/ferrigura/issues/153)) ([7ae27d2](https://github.com/sebastian-software/ferrigura/commit/7ae27d2bf0b174d31c4ce8eff2ab26f0898bd5a4))
* **fcose:** reuse repulsion neighborhood storage ([#144](https://github.com/sebastian-software/ferrigura/issues/144)) ([20f1f0c](https://github.com/sebastian-software/ferrigura/commit/20f1f0c4b83ce8e7d3044d217689c2fa6aeb6815))
* **flowchart:** reuse fixed geometry angles ([#172](https://github.com/sebastian-software/ferrigura/issues/172)) ([e4bedaf](https://github.com/sebastian-software/ferrigura/commit/e4bedafebab7aff5fee06e55f9cf8119f7ee8a15))
* **jsmath:** speed up correctly rounded acos ([#154](https://github.com/sebastian-software/ferrigura/issues/154)) ([96689a9](https://github.com/sebastian-software/ferrigura/commit/96689a9a5af335d774dc71f9020970eaad65dc41))
* **jsmath:** speed up fixed-point division ([#139](https://github.com/sebastian-software/ferrigura/issues/139)) ([5439b2a](https://github.com/sebastian-software/ferrigura/commit/5439b2ad376cbbe33afc626fda3cc8a124336c79))
* prepare diagram sources once per render ([#142](https://github.com/sebastian-software/ferrigura/issues/142)) ([df0b29a](https://github.com/sebastian-software/ferrigura/commit/df0b29a478afff200b694373d813ddafb77684ab))
* **radar:** reuse chart directions ([#166](https://github.com/sebastian-software/ferrigura/issues/166)) ([284a7ec](https://github.com/sebastian-software/ferrigura/commit/284a7eca61da56d830a03fe38c6f6ec94f76fbb4))
* reuse animated path lengths ([#167](https://github.com/sebastian-software/ferrigura/issues/167)) ([7ecd6cd](https://github.com/sebastian-software/ferrigura/commit/7ecd6cd52e117993b9b86d92f5c499cc5234ec36))
* reuse shaped text within a render ([#160](https://github.com/sebastian-software/ferrigura/issues/160)) ([f6fc797](https://github.com/sebastian-software/ferrigura/commit/f6fc797d5d9481c3936bcce2d18cc1f3916bd899))
* **swimlane:** reuse candidate crossing totals ([#168](https://github.com/sebastian-software/ferrigura/issues/168)) ([79b98a2](https://github.com/sebastian-software/ferrigura/commit/79b98a292c76a39212ffb85ef1c4142ead30ccf4))
* **swimlane:** reuse crossing repair geometry ([#155](https://github.com/sebastian-software/ferrigura/issues/155)) ([25c2a94](https://github.com/sebastian-software/ferrigura/commit/25c2a94da420b9afa9732674bd270e3206d58ef1))
* **swimlane:** speed up route key rounding ([#138](https://github.com/sebastian-software/ferrigura/issues/138)) ([5899e64](https://github.com/sebastian-software/ferrigura/commit/5899e64f96ca3120fa42ce7914daf818aaadfc57))
* **text:** reuse font subsets across renders ([#161](https://github.com/sebastian-software/ferrigura/issues/161)) ([22427a6](https://github.com/sebastian-software/ferrigura/commit/22427a6275331ebb07ba0cf1fadf4367ae8ca8af))
* **text:** reuse label shaping results ([#141](https://github.com/sebastian-software/ferrigura/issues/141)) ([2eed139](https://github.com/sebastian-software/ferrigura/commit/2eed139ee63d01521c2523d2787963a57376f0e3))
* **text:** skip repeated font byte comparisons ([#171](https://github.com/sebastian-software/ferrigura/issues/171)) ([6050009](https://github.com/sebastian-software/ferrigura/commit/6050009cd357ee528012f24ed9d8fe31b1b408e2))
* **venn:** skip unused text layout ([#143](https://github.com/sebastian-software/ferrigura/issues/143)) ([81d3d8f](https://github.com/sebastian-software/ferrigura/commit/81d3d8fd7fab7c8ab97ccef01c6c9ee1a2c24177))

## [0.8.1](https://github.com/sebastian-software/ferrigura/compare/v0.8.0...v0.8.1) (2026-11-21)


### Bug Fixes

* **deps:** update rust napi to v3.16.2 ([#103](https://github.com/sebastian-software/ferrigura/issues/103)) ([2415cc8](https://github.com/sebastian-software/ferrigura/commit/2415cc82228682fd8f617de78698bdf42dce6ff7))
* **deps:** update the Rust text stack together ([#114](https://github.com/sebastian-software/ferrigura/issues/114)) ([7485fa4](https://github.com/sebastian-software/ferrigura/commit/7485fa4610556581260454aa58d5794abd176f74))
* honor effective wrapping in C4 diagrams ([#118](https://github.com/sebastian-software/ferrigura/issues/118)) ([b6fab8d](https://github.com/sebastian-software/ferrigura/commit/b6fab8d062184845604c4ff7465188ca35340b4d))
* **venn:** include fmin license and attribution ([#113](https://github.com/sebastian-software/ferrigura/issues/113)) ([9346f2e](https://github.com/sebastian-software/ferrigura/commit/9346f2eaf37c423a35b6310b38a3bb21dba10b5b))

## [0.8.0](https://github.com/sebastian-software/ferrigura/compare/v0.7.0...v0.8.0) (2026-11-19)


### Features

* expose a fixed render clock ([#111](https://github.com/sebastian-software/ferrigura/issues/111)) ([7560d14](https://github.com/sebastian-software/ferrigura/commit/7560d14d564d4f8f82854fec4916514598829eaa))


### Bug Fixes

* enforce host input and edge limits ([#109](https://github.com/sebastian-software/ferrigura/issues/109)) ([3047a07](https://github.com/sebastian-software/ferrigura/commit/3047a07feef6d155c255946bf92e5a237cd95b89))

## [0.7.0](https://github.com/sebastian-software/ferrigura/compare/v0.6.0...v0.7.0) (2026-11-10)


### Features

* add a general compat option and fix diagram styling ([#85](https://github.com/sebastian-software/ferrigura/issues/85)) ([ce5d30f](https://github.com/sebastian-software/ferrigura/commit/ce5d30fed54325fefe51d13f16bb8b313114e124))
* **fonts:** render with host-supplied fonts across Rust, CLI and Node ([#94](https://github.com/sebastian-software/ferrigura/issues/94)) ([a9877dd](https://github.com/sebastian-software/ferrigura/commit/a9877dd25a537d1e1ca58f6deee86a8e5c8a1dd5))
* **render:** add optional SVG draw animation ([#86](https://github.com/sebastian-software/ferrigura/issues/86)) ([0966dcb](https://github.com/sebastian-software/ferrigura/commit/0966dcbb105aae94d06976618e659a28620937be))
* **text:** measure and embed explicit local fonts ([#93](https://github.com/sebastian-software/ferrigura/issues/93)) ([69f1b63](https://github.com/sebastian-software/ferrigura/commit/69f1b635485d91c74a3620fc4f0e0174359a7fef))


### Bug Fixes

* **fonts:** preserve regular metrics in sequence and journey ([#96](https://github.com/sebastian-software/ferrigura/issues/96)) ([01b8eb2](https://github.com/sebastian-software/ferrigura/commit/01b8eb239eb501c6a9ea553cf8b8857320044599))
* **render:** mask edge labels consistently across diagram types ([#95](https://github.com/sebastian-software/ferrigura/issues/95)) ([375fc03](https://github.com/sebastian-software/ferrigura/commit/375fc03bd7ce308eabc87f1c9d12bff6a5cc9941))
* **render:** match Mermaid's class and state colors ([8560f29](https://github.com/sebastian-software/ferrigura/commit/8560f29f09655a76950ade9bbf1f22da6a6fb17d))
* **site:** let the size table scroll on narrow screens ([f51b8ee](https://github.com/sebastian-software/ferrigura/commit/f51b8ee0a6790912ddfb7c7fe903cc2cf5830d94))

## [0.6.0](https://github.com/sebastian-software/ferrigura/compare/v0.5.0...v0.6.0) (2026-11-09)


### Features

* GitHub as the header's mark, on Ardo 6.4 ([#76](https://github.com/sebastian-software/ferrigura/issues/76)) ([bb2ae20](https://github.com/sebastian-software/ferrigura/commit/bb2ae20f1e83a4f3ae4673549ce487480ca8549d))


### Bug Fixes

* **mindmap:** size cloud and bang nodes as Chromium 158 maps their boxes ([a775fac](https://github.com/sebastian-software/ferrigura/commit/a775facf69d4a1e03e8631f0e6653239e6d872ab))
* **mindmap:** size cloud and bang nodes as Chromium 158 maps their boxes ([cd40108](https://github.com/sebastian-software/ferrigura/commit/cd40108ba7ecd0ab0e217cb70372129a7d19e916))
* **port:** check pie scopes against the page's color-scheme ([ab4f541](https://github.com/sebastian-software/ferrigura/commit/ab4f5417b38d9afbd45c779a19369a35938aa58e))
* **port:** check pie scopes against the page's color-scheme ([ef4d988](https://github.com/sebastian-software/ferrigura/commit/ef4d98895a3c156ddf63bdfa640f3d2fb8f6e6ab))

## [0.5.0](https://github.com/sebastian-software/ferrigura/compare/v0.4.0...v0.5.0) (2026-11-08)


### ⚠ BREAKING CHANGES

* **render:** an inline diagram in a page that declares no color-scheme now stays light instead of following the operating system. Pages with a dark mode should declare color-scheme.

### Features

* **homepage:** adopt the Monografie page system ([0657a0f](https://github.com/sebastian-software/ferrigura/commit/0657a0f044337f135e9c98ad54961a911351411b))
* **homepage:** adopt the Monografie page system ([bf79617](https://github.com/sebastian-software/ferrigura/commit/bf79617ea9b757b1143cd42c9e0468aa956d039d))
* **homepage:** show documentation sections in the Ferra section rail ([4a8ad09](https://github.com/sebastian-software/ferrigura/commit/4a8ad0943493cbdab5ed816a408217087d5380b2))
* **homepage:** show documentation sections in the Ferra section rail ([1f4b99c](https://github.com/sebastian-software/ferrigura/commit/1f4b99c395cdf859cd0da70c843fdd5a9e739834))
* let a program register only the diagram types it draws ([d46bc10](https://github.com/sebastian-software/ferrigura/commit/d46bc102c05500efe60e9aaab1ef98c217a868f7))
* let a program register only the diagram types it draws ([350566f](https://github.com/sebastian-software/ferrigura/commit/350566fc38aa9d2a156610dc23ada3f2c420b3d8))
* pick the diagram types of the Node addon with cargo features ([b25691b](https://github.com/sebastian-software/ferrigura/commit/b25691bbee1b5d6a32891ba01b74814f3b69d09f))
* pick the diagram types of the Node addon with cargo features ([c4ed46f](https://github.com/sebastian-software/ferrigura/commit/c4ed46f2c3f176148ff6b51ec2d424112cf06046))
* **render:** follow the page's color-scheme with light-dark() ([75527d9](https://github.com/sebastian-software/ferrigura/commit/75527d9d94dfe7b221082626fa88a054d51a8e3e))


### Performance Improvements

* **build:** cut the shipped binaries by 35 to 45 percent ([8869152](https://github.com/sebastian-software/ferrigura/commit/8869152faba508ab0b25bcf45b1e764df2af29aa))
* **build:** cut the shipped binaries by 35 to 45 percent ([1f09a90](https://github.com/sebastian-software/ferrigura/commit/1f09a903b7aee2451f01922a441b3c9846266e0b))
* **deps:** drop the regex crate for 1.2 MB smaller binaries ([8561294](https://github.com/sebastian-software/ferrigura/commit/8561294676f9d13d2dce81535185e3aacbe2fe38))
* **deps:** drop the regex crate for 1.2 MB smaller binaries ([7775af6](https://github.com/sebastian-software/ferrigura/commit/7775af65b45ab9733afbae0d7c5cf3794a3721ff))
* read only the configuration sections a diagram type renders with ([d9e5087](https://github.com/sebastian-software/ferrigura/commit/d9e508722583596fc219550fb74945f9fa2a7817))
* read only the configuration sections a diagram type renders with ([e04ebbf](https://github.com/sebastian-software/ferrigura/commit/e04ebbf8f916fbe9923a8210271e6a832b48c01c))

## [0.4.0](https://github.com/sebastian-software/ferrigura/compare/v0.3.1...v0.4.0) (2026-11-04)


### Features

* **flowchart:** improve default routing and label backgrounds ([#19](https://github.com/sebastian-software/ferrigura/issues/19)) ([3714ca8](https://github.com/sebastian-software/ferrigura/commit/3714ca8e13d9732e34eb93973e957af08b731215))

## [0.3.1](https://github.com/sebastian-software/ferrigura/compare/v0.3.0...v0.3.1) (2026-11-04)


### Bug Fixes

* **deps:** update Rust napi to v3.16.1 ([897d309](https://github.com/sebastian-software/ferrigura/commit/897d3090f505510a359cdaeaa9acbad9c186338d))

## 0.3.0 (2026-10-27)

The first public baseline. ferrigura renders the diagram text of all 33 diagram types of Mermaid 12.2.0 to SVG, in Rust and without a browser:

* the library `ferrigura` and the command-line tool `ferrigura-cli` on crates.io, and a Node.js addon with its npm packages in `node/`
* the layouts ported from their sources: ELK Layered, cose-bilkent, fcose, the tidy tree, swimlanes and Venn diagrams
* text measured with embedded fonts, so the output is the same on every machine
* parsers checked against Mermaid 12.2.0 model by model, and the geometry of the output checked against Mermaid 12.2.0 in Chromium
