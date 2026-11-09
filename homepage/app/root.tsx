import type { ReactNode } from 'ardo/react';

import { ArdoActiveLink as NavLink } from 'ardo/runtime';
import {
  MonoArdoErrorBoundary,
  MonoArdoLayout,
  MonoArdoSite,
  type MonoDocsSection,
} from 'ferramenta-family/ardo';
import touchIcon from 'ferramenta-family/ferra-assets/ferrigura/apple-touch-icon.png?url';
import favicon from 'ferramenta-family/ferra-assets/ferrigura/favicon.ico?url';

import navigation from './data/navigation.json';
import { version } from './version';
import 'ferramenta-family/monografie.css';
import 'ferramenta-family/monografie-docs.css';
import 'ferramenta-family/monografie-ardo.css';

// Ferrigura's own specimens, all under fgr- classes; after the package so they win its ties.
import './components/homepage-specimens.css';

export function Layout({ children }: { children: ReactNode }) {
  return (
    <MonoArdoLayout icon={favicon} appleTouchIcon={touchIcon}>
      {children}
    </MonoArdoLayout>
  );
}
export const ErrorBoundary = MonoArdoErrorBoundary;

const sections: MonoDocsSection[] = navigation.map((section) => ({
  id: section.id,
  label: section.label,
  to: section.to,
  icon: section.icon,
  pages: section.pages.map(({ title, to }) => ({ label: title, to })),
}));

const links = [
  <NavLink key="guide" to="/guide/getting-started">
    Guide
  </NavLink>,
  <NavLink key="reference" to="/reference/rust">
    Reference
  </NavLink>,
  <NavLink key="diagrams" to="/diagrams/overview">
    Diagram types
  </NavLink>,
  <NavLink key="comparisons" to="/comparisons/overview">
    Comparisons
  </NavLink>,
];

/** Every page on Monografie's Ardo shell; the documentation adds its menu, search and reading layout. */
export default function Root() {
  return (
    <MonoArdoSite
      product="ferrigura"
      name="Ferrigura"
      release={version}
      links={links}
      repository="https://github.com/sebastian-software/ferrigura"
      sections={sections}
      homeClassName="fgr-site"
      docsClassName="fgr-docs"
      footer={
        <>
          <a href="https://github.com/sebastian-software/ferrigura">GitHub</a>
          <a href="https://crates.io/crates/ferrigura">crates.io</a>
          <a href="https://www.npmjs.com/package/@ferrigura/core">npm</a>
          <a href="/notices/diagram-fonts.txt">Diagram font licenses</a>
          <a href="https://ardo-docs.dev">Built with Ardo</a>
        </>
      }
    />
  );
}
