# Ferrigura native distribution notices

This distribution contains the Ferrigura engine, its layouts, and bundled fonts.
The following original notices and license texts accompany every native platform package.


## crates/ferrigura-cli/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-cose/LICENSE

MIT License

Copyright (c) 2016-2018, The Cytoscape Consortium (cytoscape-cose-bilkent 4.1.0)
Copyright (c) 2019 - present, iVis@Bilkent (cose-base 1.0.3)
Copyright (c) 2019 iVis@Bilkent (layout-base 1.0.2)
Copyright (c) 2018 - present, iVis-at-Bilkent (cytoscape-fcose 2.2.0)
Copyright (c) 2019 - present, iVis@Bilkent (cose-base 2.2.0)
Copyright (c) 2019 iVis@Bilkent (layout-base 2.0.1)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-cose/NOTICE.md

# Notice: sources and licences of ferrigura-cose

The crate is MIT-licensed, as its sources are. It ports two engines, both MIT packages at the
versions Mermaid's bundle loads. The cose-bilkent engine (`src/` outside `fcose/`) ports the tree
path of three packages:

| Package                  | Version | Copyright                                         | Ported in                                                                                                                                                                                             |
| ------------------------ | ------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-cose-bilkent` | 4.1.0   | Copyright (c) 2016-2018, The Cytoscape Consortium | `src/graph.rs`, `src/engine.rs` (`index.js`: the adapter and `getUserOptions`)                                                                                                                        |
| `cose-base`              | 1.0.3   | Copyright (c) 2019 - present, iVis@Bilkent        | `src/forest.rs`, `src/forces.rs`, `src/engine.rs` (`CoSELayout.js`, `CoSENode.js`)                                                                                                                    |
| `layout-base`            | 1.0.2   | Copyright (c) 2019 iVis@Bilkent                   | `src/geometry.rs`, `src/graph.rs`, `src/forces.rs`, `src/forest.rs` (`Layout.js`, `LGraph.js`, `LNode.js`, `LEdge.js`, `FDLayout.js`, `util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`) |

The fcose engine (`src/fcose/`, the architecture diagram's layout) ports:

| Package           | Version | Copyright                                     | Ported in                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------- | ------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cytoscape-fcose` | 2.2.0   | Copyright (c) 2018 - present, iVis-at-Bilkent | `src/fcose/mod.rs` (`index.js` `run`, `cose.js` `coseLayout`, `auxiliary.js` `relocateComponent`, `processChildrenList`, `processEdges`, `calcBoundingBox`), `src/fcose/spectral.rs` (`spectral.js`, the spectral start of `randomize: true`)                                                                                                                                                                         |
| `cose-base`       | 2.2.0   | Copyright (c) 2019 - present, iVis@Bilkent    | `src/fcose/layout.rs` (`CoSELayout.js` `classicLayout`, `tick`, `moveNodes`, `initSpringEmbedder`), `src/fcose/forces.rs` (`FDLayout` forces as cose-base uses them), `src/fcose/model.rs` (`CoSENode.js`, `LGraph` and `LGraphManager` as cose-base uses them), `src/fcose/constraints.rs` (`ConstraintHandler.js`, `initConstraintVariables`, `updateDisplacements`, and the transform block for `randomize: true`) |
| `layout-base`     | 2.0.1   | Copyright (c) 2019 iVis@Bilkent               | `src/fcose/model.rs` (`LGraph.js`, `LGraphManager.js`, `LNode.js`, `LEdge.js`), `src/fcose/forces.rs` (`fd/FDLayout.js`), `src/fcose/linalg.rs` (`util/Matrix.js`, `util/SVD.js`, and the orthogonal Procrustes fit of `ConstraintHandler.js` 578-593), `src/geometry.rs` (`util/IGeometry.js`, `util/RectangleD.js`, `util/Transform.js`, `util/IMath.js`, the same functions as in 1.0.2)                           |

`src/fcose/rng.rs` reproduces the generator of Mermaid's own `architectureSeed.ts` (not a third-party
package).

The three cose-bilkent packages are under the MIT licence, and so are the fcose packages. The text
and the copyright notices of all six are in `LICENSE`.

The trigonometric and logarithm functions come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`.



## crates/ferrigura-elk/LICENSE

# Eclipse Public License - v 2.0

THE ACCOMPANYING PROGRAM IS PROVIDED UNDER THE TERMS OF THIS ECLIPSE
PUBLIC LICENSE (“AGREEMENT”). ANY USE, REPRODUCTION OR DISTRIBUTION OF
THE PROGRAM CONSTITUTES RECIPIENT'S ACCEPTANCE OF THIS AGREEMENT.

## 1. DEFINITIONS

“Contribution” means:

-   a\) in the case of the initial Contributor, the initial content
    Distributed under this Agreement, and
-   b\) in the case of each subsequent Contributor:
    -   i\) changes to the Program, and
    -   ii\) additions to the Program;

    where such changes and/or additions to the Program originate from
    and are Distributed by that particular Contributor. A Contribution
    “originates” from a Contributor if it was added to the Program by
    such Contributor itself or anyone acting on such Contributor's
    behalf. Contributions do not include changes or additions to the
    Program that are not Modified Works.

“Contributor” means any person or entity that Distributes the Program.

“Licensed Patents” mean patent claims licensable by a Contributor which
are necessarily infringed by the use or sale of its Contribution alone
or when combined with the Program.

“Program” means the Contributions Distributed in accordance with this
Agreement.

“Recipient” means anyone who receives the Program under this Agreement
or any Secondary License (as applicable), including Contributors.

“Derivative Works” shall mean any work, whether in Source Code or other
form, that is based on (or derived from) the Program and for which the
editorial revisions, annotations, elaborations, or other modifications
represent, as a whole, an original work of authorship.

“Modified Works” shall mean any work in Source Code or other form that
results from an addition to, deletion from, or modification of the
contents of the Program, including, for purposes of clarity any new file
in Source Code form that contains any contents of the Program. Modified
Works shall not include works that contain only declarations,
interfaces, types, classes, structures, or files of the Program solely
in each case in order to link to, bind by name, or subclass the Program
or Modified Works thereof.

“Distribute” means the acts of a) distributing or b) making available in
any manner that enables the transfer of a copy.

“Source Code” means the form of a Program preferred for making
modifications, including but not limited to software source code,
documentation source, and configuration files.

“Secondary License” means either the GNU General Public License, Version
2.0, or any later versions of that license, including any exceptions or
additional permissions as identified by the initial Contributor.

## 2. GRANT OF RIGHTS

-   a\) Subject to the terms of this Agreement, each Contributor hereby
    grants Recipient a non-exclusive, worldwide, royalty-free copyright
    license to reproduce, prepare Derivative Works of, publicly display,
    publicly perform, Distribute and sublicense the Contribution of such
    Contributor, if any, and such Derivative Works.
-   b\) Subject to the terms of this Agreement, each Contributor hereby
    grants Recipient a non-exclusive, worldwide, royalty-free patent license
    under Licensed Patents to make, use, sell, offer to sell, import and
    otherwise transfer the Contribution of such Contributor, if any, in
    Source Code or other form. This patent license shall apply to the
    combination of the Contribution and the Program if, at the time the
    Contribution is added by the Contributor, such addition of the
    Contribution causes such combination to be covered by the
    Licensed Patents. The patent license shall not apply to any other
    combinations which include the Contribution. No hardware per se is
    licensed hereunder.
-   c\) Recipient understands that although each Contributor grants the
    licenses to its Contributions set forth herein, no assurances are
    provided by any Contributor that the Program does not infringe the
    patent or other intellectual property rights of any other entity. Each
    Contributor disclaims any liability to Recipient for claims brought by
    any other entity based on infringement of intellectual property rights
    or otherwise. As a condition to exercising the rights and licenses
    granted hereunder, each Recipient hereby assumes sole responsibility to
    secure any other intellectual property rights needed, if any. For
    example, if a third party patent license is required to allow Recipient
    to Distribute the Program, it is Recipient's responsibility to acquire
    that license before distributing the Program.
-   d\) Each Contributor represents that to its knowledge it has sufficient
    copyright rights in its Contribution, if any, to grant the copyright
    license set forth in this Agreement.
-   e\) Notwithstanding the terms of any Secondary License, no Contributor
    makes additional grants to any Recipient (other than those set forth in
    this Agreement) as a result of such Recipient's receipt of the Program
    under the terms of a Secondary License (if permitted under the terms of
    Section 3).

## 3. REQUIREMENTS

3.1 If a Contributor Distributes the Program in any form, then:

-   a\) the Program must also be made available as Source Code, in accordance
    with section 3.2, and the Contributor must accompany the Program with a
    statement that the Source Code for the Program is available under this
    Agreement, and informs Recipients how to obtain it in a reasonable
    manner on or through a medium customarily used for software exchange;
    and
-   b\) the Contributor may Distribute the Program under a license different
    than this Agreement, provided that such license:
    -   i\) effectively disclaims on behalf of all other Contributors all
        warranties and conditions, express and implied, including warranties or
        conditions of title and non-infringement, and implied warranties or
        conditions of merchantability and fitness for a particular purpose;
    -   ii\) effectively excludes on behalf of all other Contributors all
        liability for damages, including direct, indirect, special, incidental
        and consequential damages, such as lost profits;
    -   iii\) does not attempt to limit or alter the recipients' rights in the
        Source Code under section 3.2; and
    -   iv\) requires any subsequent distribution of the Program by any party to
        be under a license that satisfies the requirements of this section 3.

3.2 When the Program is Distributed as Source Code:

-   a\) it must be made available under this Agreement, or if the Program (i)
    is combined with other material in a separate file or files made
    available under a Secondary License, and (ii) the initial Contributor
    attached to the Source Code the notice described in Exhibit A of this
    Agreement, then the Program may be made available under the terms of
    such Secondary Licenses, and
-   b\) a copy of this Agreement must be included with each copy of
    the Program.

3.3 Contributors may not remove or alter any copyright, patent,
trademark, attribution notices, disclaimers of warranty, or limitations
of liability (‘notices’) contained within the Program from any copy of
the Program which they Distribute, provided that Contributors may add
their own appropriate notices.

## 4. COMMERCIAL DISTRIBUTION

Commercial distributors of software may accept certain responsibilities
with respect to end users, business partners and the like. While this
license is intended to facilitate the commercial use of the Program, the
Contributor who includes the Program in a commercial product offering
should do so in a manner which does not create potential liability for
other Contributors. Therefore, if a Contributor includes the Program in
a commercial product offering, such Contributor (“Commercial
Contributor”) hereby agrees to defend and indemnify every other
Contributor (“Indemnified Contributor”) against any losses, damages and
costs (collectively “Losses”) arising from claims, lawsuits and other
legal actions brought by a third party against the Indemnified
Contributor to the extent caused by the acts or omissions of such
Commercial Contributor in connection with its distribution of the
Program in a commercial product offering. The obligations in this
section do not apply to any claims or Losses relating to any actual or
alleged intellectual property infringement. In order to qualify, an
Indemnified Contributor must: a) promptly notify the Commercial
Contributor in writing of such claim, and b) allow the Commercial
Contributor to control, and cooperate with the Commercial Contributor
in, the defense and any related settlement negotiations. The Indemnified
Contributor may participate in any such claim at its own expense.

For example, a Contributor might include the Program in a commercial
product offering, Product X. That Contributor is then a Commercial
Contributor. If that Commercial Contributor then makes performance
claims, or offers warranties related to Product X, those performance
claims and warranties are such Commercial Contributor's responsibility
alone. Under this section, the Commercial Contributor would have to
defend claims against the other Contributors related to those
performance claims and warranties, and if a court requires any other
Contributor to pay any damages as a result, the Commercial Contributor
must pay those damages.

## 5. NO WARRANTY {#warranty}

EXCEPT AS EXPRESSLY SET FORTH IN THIS AGREEMENT, AND TO THE EXTENT
PERMITTED BY APPLICABLE LAW, THE PROGRAM IS PROVIDED ON AN “AS IS”
BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, EITHER EXPRESS OR
IMPLIED INCLUDING, WITHOUT LIMITATION, ANY WARRANTIES OR CONDITIONS OF
TITLE, NON-INFRINGEMENT, MERCHANTABILITY OR FITNESS FOR A PARTICULAR
PURPOSE. Each Recipient is solely responsible for determining the
appropriateness of using and distributing the Program and assumes all
risks associated with its exercise of rights under this Agreement,
including but not limited to the risks and costs of program errors,
compliance with applicable laws, damage to or loss of data, programs or
equipment, and unavailability or interruption of operations.

## 6. DISCLAIMER OF LIABILITY {#disclaimer}

EXCEPT AS EXPRESSLY SET FORTH IN THIS AGREEMENT, AND TO THE EXTENT
PERMITTED BY APPLICABLE LAW, NEITHER RECIPIENT NOR ANY CONTRIBUTORS
SHALL HAVE ANY LIABILITY FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING WITHOUT LIMITATION LOST
PROFITS), HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OR DISTRIBUTION OF THE PROGRAM OR THE
EXERCISE OF ANY RIGHTS GRANTED HEREUNDER, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGES.

## 7. GENERAL

If any provision of this Agreement is invalid or unenforceable under
applicable law, it shall not affect the validity or enforceability of
the remainder of the terms of this Agreement, and without further action
by the parties hereto, such provision shall be reformed to the minimum
extent necessary to make such provision valid and enforceable.

If Recipient institutes patent litigation against any entity (including
a cross-claim or counterclaim in a lawsuit) alleging that the Program
itself (excluding combinations of the Program with other software or
hardware) infringes such Recipient's patent(s), then such Recipient's
rights granted under Section 2(b) shall terminate as of the date such
litigation is filed.

All Recipient's rights under this Agreement shall terminate if it fails
to comply with any of the material terms or conditions of this Agreement
and does not cure such failure in a reasonable period of time after
becoming aware of such noncompliance. If all Recipient's rights under
this Agreement terminate, Recipient agrees to cease use and distribution
of the Program as soon as reasonably practicable. However, Recipient's
obligations under this Agreement and any licenses granted by Recipient
relating to the Program shall continue and survive.

Everyone is permitted to copy and distribute copies of this Agreement,
but in order to avoid inconsistency the Agreement is copyrighted and may
only be modified in the following manner. The Agreement Steward reserves
the right to publish new versions (including revisions) of this
Agreement from time to time. No one other than the Agreement Steward has
the right to modify this Agreement. The Eclipse Foundation is the
initial Agreement Steward. The Eclipse Foundation may assign the
responsibility to serve as the Agreement Steward to a suitable separate
entity. Each new version of the Agreement will be given a distinguishing
version number. The Program (including Contributions) may always be
Distributed subject to the version of the Agreement under which it was
received. In addition, after a new version of the Agreement is
published, Contributor may elect to Distribute the Program (including
its Contributions) under the new version.

Except as expressly stated in Sections 2(a) and 2(b) above, Recipient
receives no rights or licenses to the intellectual property of any
Contributor under this Agreement, whether expressly, by implication,
estoppel or otherwise. All rights in the Program not expressly granted
under this Agreement are reserved. Nothing in this Agreement is intended
to be enforceable by any entity that is not a Contributor or Recipient.
No third-party beneficiary rights are created under this Agreement.

## Exhibit A – Form of Secondary Licenses Notice {#exhibit-a}

“This Source Code may also be made available under the following
Secondary Licenses when the conditions for such availability set forth
in the Eclipse Public License, v. 2.0 are satisfied: {name license(s),
version(s), and exceptions or additional permissions here}.”

> Simply including a copy of this Agreement, including this Exhibit A is
> not sufficient to license the Source Code under Secondary Licenses.
>
> If it is not possible or desirable to put the notice in a particular
> file, then You may include the notice in a location (such as a LICENSE
> file in a relevant directory) where a recipient would be likely to
> look for such a notice.
>
> You may add additional accurate notices of copyright ownership.



## crates/ferrigura-elk/NOTICE.md

# Notices for Eclipse Layout Kernel

This content is produced and maintained by the Eclipse Layout Kernel project.

- Project home: https://projects.eclipse.org/projects/modeling.elk

## Trademarks

Eclipse Layout Kernel is a trademark of the Eclipse Foundation.

## Copyright

All content is the property of the respective authors or their employers. For
more information regarding authorship of content, please consult the listed
source code repository logs.

## Declared Project Licenses

This program and the accompanying materials are made available under the
terms of the Eclipse Public License 2.0 which is available at
http://www.eclipse.org/legal/epl-2.0.

SPDX-License-Identifier: EPL-2.0

## Source Code

The project maintains the following source code repositories:

- https://github.com/eclipse/elk
- https://github.com/eclipse/elk-models

## Third-party Content

This project leverages the following third party content.

ANTLR Runtime only: (3.2)

- License: New BSD license

Apache Commons Logging Jar (1.2.0)

- License: Apache License, 2.0

Apache Log4J (1.2.24)

- License: Apache License, 2.0

ASM (9.5.0)

- License: New BSD license

Google Guava (30.1.0)

- License: Apache License 2.0

Google Guava (33.0.0)

- License: Apache License 2.0

Google Guice / Inject Core API (5.0.1)

- License: Apache License, 2.0

ICU4J (74.2)

- License: ICU

libavoid (0.3.1)

- License: GNU Lesser General Public License, 2.1

Ptolemy demo models (n/a)

- License: Ptolemy License (Custom)

## Cryptography

Content may contain encryption software. The country in which you are currently
may have restrictions on the import, possession, and use, and/or re-export to
another country, of encryption software. BEFORE using any encryption software,
please check the country's laws, regulations and policies concerning the import,
possession, or use, and re-export of encryption software, to see if this is
permitted.



## crates/ferrigura-jsmath/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-jsmath/LICENSE-LLVM

==============================================================================
The LLVM Project is under the Apache License v2.0 with LLVM Exceptions:
==============================================================================

                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

    Copyright [yyyy] [name of copyright owner]

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.


---- LLVM Exceptions to the Apache 2.0 License ----

As an exception, if, as a result of your compiling your source code, portions
of this Software are embedded into an Object form of such source code, you
may redistribute such embedded portions in such Object form without complying
with the conditions of Sections 4(a), 4(b) and 4(d) of the License.

In addition, if you combine or link compiled forms of this Software with
software that is licensed under the GPLv2 ("Combined Software") and if a
court of competent jurisdiction determines that the patent provision (Section
3), the indemnity provision (Section 9) or other Section of the License
conflicts with the conditions of the GPLv2, you may retroactively and
prospectively choose to deem waived or otherwise exclude such Section(s) of
the License, but only in their entirety and only with respect to the Combined
Software.

==============================================================================
Software from third parties included in the LLVM Project:
==============================================================================
The LLVM Project contains third party software which is under different license
terms. All such code will be identified clearly using at least one of two
mechanisms:
1) It will be in a separate directory tree with its own `LICENSE.txt` or
   `LICENSE` file at the top containing the specific license and restrictions
   which apply to that software, or
2) It will contain specific license and restriction terms at the top of every
   file.

==============================================================================
Legacy LLVM License (https://llvm.org/docs/DeveloperPolicy.html#legacy):
==============================================================================
University of Illinois/NCSA
Open Source License

Copyright (c) 2003-2019 University of Illinois at Urbana-Champaign.
All rights reserved.

Developed by:

    LLVM Team

    University of Illinois at Urbana-Champaign

    http://llvm.org

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal with
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

    * Redistributions of source code must retain the above copyright notice,
      this list of conditions and the following disclaimers.

    * Redistributions in binary form must reproduce the above copyright notice,
      this list of conditions and the following disclaimers in the
      documentation and/or other materials provided with the distribution.

    * Neither the names of the LLVM Team, University of Illinois at
      Urbana-Champaign, nor the names of its contributors may be used to
      endorse or promote products derived from this Software without specific
      prior written permission.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
CONTRIBUTORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS WITH THE
SOFTWARE.




## crates/ferrigura-jsmath/LICENSE-V8

This license applies to all parts of V8 that are not externally
maintained libraries.  The externally maintained libraries used by V8
are:

  - PCRE test suite, located in
    test/mjsunit/third_party/regexp-pcre/regexp-pcre.js.  This is based on the
    test suite from PCRE-7.3, which is copyrighted by the University
    of Cambridge and Google, Inc.  The copyright notice and license
    are embedded in regexp-pcre.js.

  - Layout tests, located in test/mjsunit/third_party/object-keys.  These are
    based on layout tests from webkit.org which are copyrighted by
    Apple Computer, Inc. and released under a 3-clause BSD license.

  - Strongtalk assembler, the basis of the files assembler-arm-inl.h,
    assembler-arm.cc, assembler-arm.h, assembler-ia32-inl.h,
    assembler-ia32.cc, assembler-ia32.h, assembler-x64-inl.h,
    assembler-x64.cc, assembler-x64.h, assembler.cc and assembler.h.
    This code is copyrighted by Sun Microsystems Inc. and released
    under a 3-clause BSD license.

  - Valgrind client API header, located at third_party/valgrind/valgrind.h
    This is released under the BSD license.

  - The Wasm C/C++ API headers, located at third_party/wasm-api/wasm.{h,hh}
    This is released under the Apache license. The API's upstream prototype
    implementation also formed the basis of V8's implementation in
    src/wasm/c-api.cc.

These libraries have their own licenses; we recommend you read them,
as their terms may differ from the terms below.

Further license information can be found in LICENSE files located in
sub-directories.

Copyright 2014, the V8 project authors. All rights reserved.
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.
    * Neither the name of Google Inc. nor the names of its
      contributors may be used to endorse or promote products derived
      from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.



## crates/ferrigura-jsmath/NOTICE.md

# Notice: sources and licences of ferrigura-jsmath

The crate is MIT-licensed (see `LICENSE`). Parts of it are ported from other sources, and each part
keeps the licence of its source. `LICENSE-LLVM` and `LICENSE-V8` carry the licence texts.

## The default functions (Chromium 151)

| Function                                            | Where                                                                        | Source                                                                                                                                                                        | Licence                                         | Depends on the target's fused multiply-adds?                                                              |
| --------------------------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `sin`, `cos`, `tan`, `asin`, `acos`, `log`, `log10` | `src/correct.rs`, constants in `src/constants.rs` (`CR_*`)                   | Written for this crate: a correctly rounded evaluator (Ziv's method, 384-bit fixed point). No code is copied. The constants were computed with mpmath and Python's `decimal`. | MIT (this crate)                                | No. The fixed-point arithmetic is integer arithmetic, and the results are the correctly rounded values.   |
| `atan`, `atan2`                                     | `src/atan.rs` (the table `ATAN_I`, `atan_eval`, `dd_div`, the special cases) | llvm-libc: `libc/src/__support/math/atan.h`, `atan2.h`, `atan_utils.h`, and the `FPUtil` helpers they use. Reproduced step by step.                                           | Apache-2.0 WITH LLVM-exception (`LICENSE-LLVM`) | Yes. llvm-libc uses fused multiply-adds where its `LIBC_TARGET_CPU_HAS_FMA` is set, which is `CONTRACTS`. |
| `pow`                                               | `src/pow.rs`                                                                 | V8's `src/numbers/ieee754.cc` (the special cases around `std::pow`), and the platform's `pow`                                                                                 | BSD-style (`LICENSE-V8`)                        | The platform's `pow` decides it.                                                                          |
| `hypot`                                             | `src/hypot.rs`                                                               | V8's `src/builtins/math.tq` (`FastMathHypot`, the two-argument form)                                                                                                          | BSD-style (`LICENSE-V8`)                        | No.                                                                                                       |

## The `node` module (V8 13.6, Node 24)

| Function                                                             | Where                                                                                                                                      | Source                                                                                                                   | Licence                                        | Depends on the target's fused multiply-adds?                                                        |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `log`, `log10` | `src/trig.rs`, `src/inverse.rs`, `src/log.rs`, `src/rem_pio2.rs`, constants in `src/constants.rs` (fdlibm), `src/fused.rs`, `src/words.rs` | V8's `src/base/ieee754.cc` (V8 13.6.233.17), adapted from fdlibm (Sun Microsystems, 1993; the notice is in `src/lib.rs`) | BSD-style (`LICENSE-V8`) and the fdlibm notice | Yes: V8's compiler fuses them on targets that have fused multiply-adds, and `CONTRACTS` follows it. |
| `pow`, `hypot`                                                       | `src/pow.rs`, `src/hypot.rs`                                                                                                               | As above                                                                                                                 | BSD-style (`LICENSE-V8`)                       | As above                                                                                            |

`src/words.rs`, `scalbn`, follows the algorithm of musl's `scalbn` (MIT, Rich Felker and the musl
contributors). It is written out again here, not copied.

## Test data

`tests/data/chromium` holds Chromium 151's results and `tests/data/node` Node 24's, for samples of
the probe's inputs (`tools/port/jsmath-probe.ts`). The data are outputs of the two runtimes, not
copies of any source.



## crates/ferrigura-model/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-node/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-path/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-path/LICENSE-CHROMIUM

// Copyright 2015 The Chromium Authors
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//    * Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above
// copyright notice, this list of conditions and the following disclaimer
// in the documentation and/or other materials provided with the
// distribution.
//    * Neither the name of Google LLC nor the names of its
// contributors may be used to endorse or promote products derived from
// this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.



## crates/ferrigura-path/LICENSE-SKIA

Copyright (c) 2011 Google Inc. All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

  * Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.

  * Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in
    the documentation and/or other materials provided with the
    distribution.

  * Neither the name of the copyright holder nor the names of its
    contributors may be used to endorse or promote products derived
    from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.



## crates/ferrigura-path/NOTICE.md

# Notice: sources and licences of ferrigura-path

The crate is MIT-licensed (see `LICENSE`), and its ported parts keep the licence of their sources:
`LICENSE-SKIA` (Skia's BSD-3-Clause licence) and `LICENSE-CHROMIUM` (Chromium's licence, which covers
Blink and `ui/gfx`).

The port is by reading the sources at the revisions that Chromium 151 uses (Chromium 151.0.7922.34,
and the Skia revision its `DEPS` names, `0208108c7aed5f4e0faa525cbba52c238005a1ef`). Nothing is copied
or vendored; the code follows the operations of the sources, and each module cites its file and lines.

| Module        | Source                                                                                                                                                                                                                                                                                                                              | Licence                       |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| `number.rs`   | `third_party/blink/renderer/core/svg/svg_parser_utilities.cc` (`GenericParseNumber`), `svg_path_string_source.cc` (`ParseArcFlag`)                                                                                                                                                                                                  | Chromium (`LICENSE-CHROMIUM`) |
| `segments.rs` | `core/svg/svg_path_string_source.cc` (`ParseSegment`, `MaybeImplicitCommand`), `core/svg/svg_path_byte_stream_builder.cc`                                                                                                                                                                                                           | Chromium                      |
| `builder.rs`  | `core/svg/svg_path_builder.cc` (`EmitSegment`, `SmoothControl`, `EmitArcTo`), `platform/geometry/path_builder.cc`                                                                                                                                                                                                                   | Chromium                      |
| `skia.rs`     | `src/core/SkPathBuilder.cpp` (`arcTo`, `conicTo`), `src/core/SkMatrix.cpp` (`setRotate`, `setScale`, `preScale`, `preRotate`, `setConcat`, `Affine_vpts`, `mapPointAffine`), `include/core/SkScalar.h`, `include/private/SkFloatingPoint.h`                                                                                         | Skia (`LICENSE-SKIA`)         |
| `bounds.rs`   | `src/core/SkPathPriv.cpp` (`ComputeTightBounds`), `src/core/SkGeometry.cpp` and `.h` (`SkFindUnitQuadRoots`, `SkFindCubicExtrema`, `SkFindQuadExtrema`, `conic_find_extrema`, `SkCubicCoeff`, `SkQuadCoeff`, `SkConicCoeff`), `ui/gfx/geometry/skia_conversions.cc` (`SkRectToRectF`), `ui/gfx/geometry/rect_f.cc` (`BoundingRect`) | Skia and Chromium             |

The corpus in `tests/corpus/paths.json` is generated by `tools/port/path-bounds.ts` from Chromium 151's
own `getBBox()`, with the `known-differences.json` list of the paths the port does not yet match.

The float `sin`, `cos`, `tan` and `atan2` of the arcs are correctly rounded (`skia.rs`), the same on every
host. Chromium on macOS takes them from the macOS libm (the sine and cosine from one `__sincosf_stret` call),
whose float functions are not correctly rounded for a few inputs; the 81 boxes of the corpus that differ for
it, all but one within 0.05 px, are in `known-differences.json`.



## crates/ferrigura-render/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-render/NOTICE.md

# Notice: sources and licences of ferrigura-render

The crate is MIT-licensed, as Mermaid 12.2.0 is, whose renderers it ports. It also ports part of
marked 16.3.0, the Markdown lexer Mermaid reads Markdown labels with, which is MIT-licensed too.

| Source (marked 16.3.0, `src/`) | Ported in                                                                          |
| ------------------------------ | ---------------------------------------------------------------------------------- |
| `rules.ts`                     | `src/unified/marked.rs`: the block and inline rules of the GFM grammar, verbatim   |
| `Lexer.ts`, `Tokenizer.ts`     | `src/unified/marked.rs`: the lexer and the tokenizers `markdownToLines` depends on |
| `helpers.ts`                   | `src/unified/marked.rs`: `rtrim`, `splitCells` and `findClosingBracket`            |

## marked

Copyright (c) 2018+, MarkedJS (https://github.com/markedjs/)
Copyright (c) 2011-2018, Christopher Jeffrey (https://github.com/chjj/)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.



## crates/ferrigura-swimlane/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-swimlane/NOTICE.md

# Notice: sources and licence of ferrigura-swimlane

The crate is MIT-licensed, as its sources are. It ports the swimlane layout engine of Mermaid 12.2.0,
`packages/mermaid/src/rendering-util/layout-algorithms/swimlanes/` (default path only), which is MIT
(Copyright (c) 2014 - 2022 Knut Sveidqvist, and the Mermaid contributors). The text of the licence is in
`LICENSE`, copied from the repository's root.

The port reads no third-party package. The floating-point helpers (`Math.hypot`, `Math.sin` and the rest)
come from `ferrigura-jsmath`, whose notice is in its own `NOTICE.md`. The `localeCompare` of the layout is
ICU's root collation for ASCII, written as a table in `src/js.rs` and checked against V8.



## crates/ferrigura-syntax/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-text/LICENSE

The MIT License (MIT)

Copyright (c) 2014 - 2022 Knut Sveidqvist

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-text/NOTICE.md

# Notice: the bundled fonts

The crate's sources are MIT-licensed, as the workspace is. The fonts in `fonts/` are third-party
works under the licences below. Each font file is a cut of its source (only some code points, the
hinting dropped, some name records kept); the cut is made by `pnpm -C tools/port port:cut-fonts`
(`tools/port/fonts/cut-faces.ts`), which checks each source's SHA-256 first and writes the cut with
`cut_font` (`examples/cut_font.rs`).

| Bundled file                               | What it draws                                                               | Source and version                                                                                                                                                                                         | Licence                                                           | SHA-256 of the source                                              | SHA-256 of the bundled file                                        |
| ------------------------------------------ | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `TwemojiMozilla-COLRv0-Emoji.ttf`          | Emoji with emoji presentation (1,185 of the 1,219 the Unicode property has) | Twemoji Mozilla 0.7.0, `twemoji-colr/Twemoji.Mozilla.v0.7.0.ttf` from <https://github.com/mozilla/twemoji-colr/releases/tag/v0.7.0>, last released 2022-10-28                                              | Art CC BY 4.0, code Apache-2.0: `fonts/LICENSE-TwemojiMozilla.md` | `6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e` | `ac0cb417936739fd190631e30a7c2e212b185d3122a0c2a6284331200756e143` |
| `NotoSansSymbols2-Regular-TextSymbols.ttf` | Text-default symbols of U+2600-27BF (252 code points)                       | Noto Sans Symbols 2, version 2.003, `unhinted/ttf/NotoSansSymbols2/NotoSansSymbols2-Regular.ttf` from <https://github.com/googlefonts/noto-fonts>, as it stood on 2026-10-20 (the commit was not recorded) | SIL Open Font License 1.1: `fonts/OFL-NotoSansSymbols2.txt`       | `882d142b9a1ef3fd7fa4225dbe95c10fab6664206eb4964c8ff705a4f6d02988` | `d65509d5d2637602bb1458df8bd71ccb0f67be0520f0926658904fe514fe23a4` |
| `NotoSansMath-Regular-Arrows.ttf`          | The arrows, U+2190-21FF (112 glyphs)                                        | Noto Sans Math 3.000, `googlefonts/ttf/NotoSansMath-Regular.ttf`, see `fonts/OFL-NotoSansMath.txt`                                                                                                         | SIL Open Font License 1.1: `fonts/OFL-NotoSansMath.txt`           | `fe380412e05c2cca4b712644c58ed38ed7b1e3153bc483a8b38725fa487b8830` | `e58bb5269af4dde2fb5fec207f9794b227ddae597dd56a50fd86224d72d2ed36` |

The text faces, Lexica Ultralegible and Atkinson Hyperlegible Mono, are credited in
`fonts/OFL-Lexica.txt` and `fonts/OFL-AtkinsonMono.txt`.

## Attribution

Emoji art in `TwemojiMozilla-COLRv0-Emoji.ttf` comes from Twemoji (<https://twitter.github.io/twemoji>),
used and redistributed under the Creative Commons Attribution 4.0 International licence
(<https://creativecommons.org/licenses/by/4.0/>), as the Twemoji project offers it. The font's code
(the build of Twemoji Mozilla) is Copyright 2016-2018 Mozilla Foundation, under the Apache License,
Version 2.0. The licence file of the build is `fonts/LICENSE-TwemojiMozilla.md`.

Changes made to the Twemoji art, as CC BY 4.0 asks them to be indicated: the font holds only the
1,185 emoji with emoji presentation that the source covers, its hinting is dropped, and its name
records are cut to 0 to 6, 13 and 14. The colour tables (COLR, CPAL) and the glyph outlines are
otherwise as the source has them.

The Noto Sans Symbols 2 cut is changed in the same way: 252 code points, no hinting, name records 0
to 6, 13 and 14. Its name table reads "Copyright 2017 Google Inc. All Rights Reserved."; its
licence text is `fonts/OFL-NotoSansSymbols2.txt`, which begins with the copyright line of the
package.

## Subsetting at embedding

A diagram's labels embed a subset of these faces (`embed_runs`) under a family name of their own.
A subset is a cut of the bundled file and is under the same licence as the file. The embedded
subsets hold the glyphs a diagram draws, not the whole file.



## crates/ferrigura-text/fonts/LICENSE-TwemojiMozilla.md

## License for the Code

Copyright 2016-2018, Mozilla Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## License for the Visual Design

The Emoji art in the twe-svg.zip archive comes from [Twemoji](https://twitter.github.io/twemoji),
and is used and redistributed under the CC-BY-4.0 [license terms](https://github.com/twitter/twemoji#license)
offered by the Twemoji project.

### Creative Commons Attribution 4.0 International (CC BY 4.0)

https://creativecommons.org/licenses/by/4.0/legalcode
or for the human readable summary: https://creativecommons.org/licenses/by/4.0/

#### You are free to:

**Share** — copy and redistribute the material in any medium or format

**Adapt** — remix, transform, and build upon the material for any purpose, even commercially.

The licensor cannot revoke these freedoms as long as you follow the license terms.

#### Under the following terms:

**Attribution** — You must give appropriate credit, provide a link to the license,
and indicate if changes were made.
You may do so in any reasonable manner, but not in any way that suggests the licensor endorses you or your use.

**No additional restrictions** — You may not apply legal terms or **technological measures**
that legally restrict others from doing anything the license permits.

#### Notices:

You do not have to comply with the license for elements of the material in the public domain
or where your use is permitted by an applicable exception or limitation. No warranties are given.
The license may not give you all of the permissions necessary for your intended use.
For example, other rights such as publicity, privacy, or moral rights may limit how you use the material.



## crates/ferrigura-text/fonts/OFL-AtkinsonMono.txt

Copyright 2020-2024 The Atkinson Hyperlegible Mono Project Authors (https://github.com/googlefonts/atkinson-hyperlegible-next-mono)

This Font Software is licensed under the SIL Open Font License, Version 1.1.
This license is copied below, and is also available with a FAQ at:
https://openfontlicense.org


-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded, 
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.



## crates/ferrigura-text/fonts/OFL-Lexica.txt

Copyright 2020 Braille Institute of America, Inc.

This Font Software is licensed under the SIL Open Font License, Version 1.1.
This license is copied below, and is also available with a FAQ at:
http://scripts.sil.org/OFL

-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded, 
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.



## crates/ferrigura-text/fonts/OFL-NotoSansMath.txt

Noto Sans Math 3.000: the arrows block (U+2190-21FF) in NotoSansMath-Regular-Arrows.ttf.

Source: https://github.com/notofonts/math, release NotoSansMath-v3.000 (commit
4b78263eac2301b04367d600a07b521cfbff0867 in the Google Fonts metadata), the file
googlefonts/ttf/NotoSansMath-Regular.ttf, sha256 fe380412e05c2cca4b712644c58ed38ed7b1e3153bc483a8b38725fa487b8830.
The bundled file is that font subset to U+2190-21FF (112 glyphs) with skera, the default
layout features, every script and no hinting; the name records 0 to 6, 13 and 14 are kept.

The licence text below is the OFL.txt of the same release.

Copyright 2022 The Noto Project Authors (https://github.com/notofonts/math)

This Font Software is licensed under the SIL Open Font License, Version 1.1.
This license is copied below, and is also available with a FAQ at:
https://scripts.sil.org/OFL


-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded, 
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.



## crates/ferrigura-text/fonts/OFL-NotoSansSymbols2.txt

Noto Sans Symbols 2, version 2.003: the text symbols of U+2600-27BF (NotoSansSymbols2-Regular-TextSymbols.ttf).

Source: https://github.com/googlefonts/noto-fonts, unhinted/ttf/NotoSansSymbols2/NotoSansSymbols2-Regular.ttf
(the file as it stood on 2026-10-20; the commit was not recorded), sha256
882d142b9a1ef3fd7fa4225dbe95c10fab6664206eb4964c8ff705a4f6d02988. Its name table holds
"Copyright 2017 Google Inc. All Rights Reserved." and version 2.003.
The bundled file is that font cut to the 252 code points of U+2600-27BF that are text-default
(not Emoji_Presentation) and that the font covers, with skera (`ferrigura-text`'s `cut_font`), the
default layout features, every script and no hinting; the name records 0 to 6, 13 and 14 are kept.
It is made by `pnpm -C tools/port port:cut-fonts`, which checks the source's checksum first.

The licence text below is the LICENSE of the same package, which begins
"Copyright 2018 The Noto Project Authors (github.com/googlei18n/noto-fonts)".

Copyright 2018 The Noto Project Authors (github.com/googlei18n/noto-fonts)

This Font Software is licensed under the SIL Open Font License,
Version 1.1.

This license is copied below, and is also available with a FAQ at:
http://scripts.sil.org/OFL

-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font
creation efforts of academic and linguistic communities, and to
provide a free and open framework in which fonts may be shared and
improved in partnership with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply to
any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software
components as distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to,
deleting, or substituting -- in part or in whole -- any of the
components of the Original Version, by changing formats or by porting
the Font Software to a new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed,
modify, redistribute, and sell modified and unmodified copies of the
Font Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components, in
Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the
corresponding Copyright Holder. This restriction only applies to the
primary font name as presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created using
the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.



## crates/ferrigura-venn/LICENSE

MIT License

Copyright (c) 2013 Ben Frederickson (@upsetjs/venn.js 2.0.0)
Copyright (c) 2021 Samuel Gratzl (@upsetjs/venn.js 2.0.0)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



## crates/ferrigura-venn/NOTICE.md

# Notice: sources and licences of ferrigura-venn

The crate is MIT-licensed, as its sources are. It ports the layout of `@upsetjs/venn.js` 2.0.0, an
MIT package (`LICENSE`, the copyright lines of Ben Frederickson and Samuel Gratzl), which Mermaid
12.2.0 bundles. What runs is the package's `build/venn.esm.js`, which inlines the `fmin` functions
the layout calls (build lines 262-622). The port follows that build, the code that runs.

| Source (`build/venn.esm.js`)                                                                                                                          | Lines     | Ported in                                                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | ---------------------------------------------------------------------------------------------------------- |
| `intersectionArea`, `containedInCircles`, `getIntersectionPoints`, `circleArea`, `distance`, `circleOverlap`, `circleCircleIntersection`, `getCenter` | 1-268     | `src/geometry.rs`                                                                                          |
| `bisect`, `nelderMead`, `wolfeLineSearch`, `conjugateGradient`, `dot`, `norm2`                                                                        | 262-622   | `src/fmin.rs` (the `fmin` functions as inlined)                                                            |
| `venn`                                                                                                                                                | 623-672   | `src/placement.rs` (`venn`)                                                                                |
| `distanceFromIntersectArea`, `addMissingAreas`                                                                                                        | 684-768   | `src/placement.rs` (`distance_from_intersect_area`, `add_missing_areas`)                                   |
| `getDistanceMatrices`, `constrainedMDSGradient`, `bestInitialLayout`, `constrainedMDSLayout`                                                          | 770-932   | `src/placement.rs` (`distance_matrices`, `constrained_gradient`, `best_initial_layout`, `constrained_mds`) |
| `greedyLayout`                                                                                                                                        | 934-1077  | `src/placement.rs` (`greedy`)                                                                              |
| `lossFunction` (the squared error only)                                                                                                               | 1079-1101 | `src/placement.rs` (`loss`)                                                                                |
| `orientateCircles`, `disjointCluster`, `getBoundingBox`, `normalizeSolution`, `scaleSolution`, `toObjectNotation`, `fromObjectNotation`               | 1134-1447 | `src/arrange.rs`, and `src/js.rs` (`JsObject`) for the key order                                           |
| `VennDiagram`'s pipeline: the removal of zero-size singles, `venn`, `normalizeSolution`, `scaleSolution`, `computeTextCentres`                        | 1503-1526 | `src/diagram.rs` (`chart`)                                                                                 |
| `circleMargin`, `computeTextCentre`, `getOverlappingCircles`, `computeTextCentres`                                                                    | 1874-2042 | `src/text.rs`                                                                                              |
| `layout`                                                                                                                                              | 2146-2212 | `src/diagram.rs` (`layout`), without the arcs and path strings                                             |

The JavaScript behaviours the order of results depends on are in `src/js.rs`: `Object.keys` order,
V8's `Array.prototype.sort` (binary insertion for arrays under 64 elements, which gives the same
order as V8 for every comparator, NaN included), `Math.min` and `Math.max` on NaN and signed zero,
and the UTF-16 order of strings.

## Numbers

`sin`, `cos`, `atan2` and `acos` come from `ferrigura-jsmath`, whose notice is in its own
`NOTICE.md`. They are Chromium's functions, bit for bit, which is what the reference runs in.

## fmin

The `fmin` package is a devDependency of venn.js at 0.0.4, with a yarn patch (`package.json` of the
venn.js package, line 82). Neither the package nor the patch is in the repository: only the bundle's
copy of its functions is, and that copy is what the port follows. Its licence and copyright line are
**unverified**: `LICENSE` lists only the copyright of venn.js.

## The corpus

`tests/corpus` holds the cases written by `tools/port/crates/venn/capture-corpus.ts`:

- 21 fixtures: every file of `e2e/diagrams/venn`, parsed with the reference's own Jison grammar and
  DB, and given the reference's `ensurePairwiseSubsets` (`vennRenderer.ts`). The box is the renderer's
  default (800 by 450, less the title's 24 px), the layout's padding 8, the chart's padding 15.
- 26 probes: hand-written texts (numeric and alphanumeric ids, duplicates, zero and negative sizes,
  overlaps of every kind, a set named `constructor`, a set named `__proto__`) and area lists given to
  the library directly (no areas, an area with no single, an empty area).
- 400 generated set systems (`tools/generate-set-systems.ts`, seed 20261018): two to six sets, the
  five id schemes, sizes of every kind, duplicated sets and unions, through the same parser and
  renderer function. They cycle through four boxes and layout paddings.

The reference runs in Chromium through the geometry harness (`withPage` in
`tools/port/geometry/common.ts`). The library's build is served to the page with one line appended
that exports `addMissingAreas` and `constrainedMDSLayout`; the build exports the rest. The chart's
drawing needs d3 and is not run: its pipeline is replicated from the build's exported functions
(the lines above). `layout()` is called as the build exports it.

Random numbers: `Math.random` is replaced by the harness's generator (mulberry32, `renderReference`
in `tools/port/geometry/common.ts`), and each draw is recorded. The stages and the chart each start
the generator at seed 1, as a render does; the layout continues from the chart's draws, as the
renderer's second call does, and starts again from seed 1 only when the chart throws. The tests seed
a port of that generator the same way, and check that its draws are the recorded ones.

## Not ported

The options the renderer does not pass: `distinct`, `scaleToFit`, `symmetricalTextCentre`,
`orientationOrder`, the logarithmic and custom losses, custom layouts, the weight of an area, the
history options, `restarts` and `maxIterations` (fixed at 10 and 500, as `venn()` sets them). The
path strings, arcs and `distinctPath` of `layout()`. The drawing. The `console.log` warnings. A set
named `__proto__` is refused (`Error::ReservedName`).


