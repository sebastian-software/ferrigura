import { type ReactNode, useState } from 'ardo/react';
import { ArdoActiveLink as NavLink, useArdoPageData, useArdoPath } from 'ardo/runtime';
import {
  ArdoErrorBoundary,
  ArdoHeaderSearch,
  ArdoMobileNavigation,
  ArdoRoot,
  ArdoRootLayout,
  ArdoTOC,
} from 'ardo/ui';
import { Mark, MarkDefs } from 'ferramenta-family';
import touchIcon from 'ferramenta-family/ferra-assets/ferrigura/apple-touch-icon.png?url';
import favicon from 'ferramenta-family/ferra-assets/ferrigura/favicon.ico?url';
import {
  MonoFooter,
  MonoGuideNavigation,
  MonoHeader,
  MonoPage,
  MonoReadingLayout,
  MonoReadingTable,
  MonoSectionRail,
} from 'ferramenta-family/monografie';
import 'ferramenta-family/monografie.css';
import 'ferramenta-family/monografie-docs.css';
import 'ferramenta-family/monografie-ardo.css';

import navigation from './data/navigation.json';
import { version } from './version';
// Ferrigura's own specimens, all under fgr- classes; after the package so they win its ties.
import './components/homepage-specimens.css';

export function Layout({ children }: { children: ReactNode }) {
  return (
    <ArdoRootLayout
      className="mono-ardo"
      colorScheme="light"
      icon={favicon}
      appleTouchIcon={touchIcon}
      iconBasePath={false}
    >
      {children}
    </ArdoRootLayout>
  );
}
export const ErrorBoundary = ArdoErrorBoundary;

function GuideNavigation({ onNavigate }: { onNavigate?: () => void }) {
  const path = useArdoPath();
  const first = navigation[0];
  if (!first) throw new Error('Documentation navigation is empty.');
  const current =
    navigation.find((section) => path === `/${section.id}` || path.startsWith(`/${section.id}/`)) ??
    first;
  return (
    <>
      <MonoSectionRail aria-label="Documentation sections">
        {navigation.map((section) => (
          <NavLink
            key={section.id}
            to={section.to}
            data-active={current.id === section.id}
            onClick={onNavigate}
          >
            <Mark name={section.icon} />
            {section.label}
          </NavLink>
        ))}
      </MonoSectionRail>
      <MonoGuideNavigation aria-label="Documentation pages">
        <NavLink to="/" onClick={onNavigate}>
          Overview
        </NavLink>
        <p>{current.label}</p>
        {current.pages.map(({ title, to: href }) => (
          <NavLink key={href} to={href} end onClick={onNavigate}>
            {title}
          </NavLink>
        ))}
      </MonoGuideNavigation>
    </>
  );
}

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
  <a key="github" href="https://github.com/sebastian-software/ferrigura">
    GitHub
  </a>,
];

/** The documentation menu and search, beside the header's own menu on documentation pages. */
function DocsActions() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <ArdoMobileNavigation title="Ferrigura documentation" open={open} onOpenChange={setOpen}>
        <GuideNavigation
          onNavigate={() => {
            setOpen(false);
          }}
        />
      </ArdoMobileNavigation>
      <ArdoHeaderSearch
        variant="icon"
        onActivate={() => {
          setOpen(false);
        }}
      />
    </>
  );
}

/** The Monografie header on every page; documentation adds its section menu and search. */
function Header({ docs }: { docs: boolean }) {
  return (
    <MonoHeader
      product="ferrigura"
      home="/"
      release={version}
      links={links}
      actions={docs ? <DocsActions /> : undefined}
    />
  );
}

function Article({ children }: { children: ReactNode }) {
  const page = useArdoPageData();
  const path = useArdoPath();
  const current = navigation.find(
    (section) => path === `/${section.id}` || path.startsWith(`/${section.id}/`)
  );
  return (
    <MonoReadingLayout
      id="ferrigura-article"
      navigation={<GuideNavigation />}
      outline={<ArdoTOC source="dom" scope="#ferrigura-article" />}
    >
      <header>
        <p>Ferrigura / {current?.label ?? 'Documentation'}</p>
        <h1>{page?.title.replaceAll('`', '') ?? 'Documentation'}</h1>
      </header>
      {children}
    </MonoReadingLayout>
  );
}
const components = { table: MonoReadingTable, wrapper: Article };

/**
 * Every page is a Monografie page: the homepage, and the documentation in the
 * Monografie reading layout.
 */
export default function Root() {
  const path = useArdoPath();
  const home = path === '/' || path === '';
  const shell = (
    <>
      <ArdoRoot
        className="mono-ardo-shell"
        scrollMode="document"
        header={<Header docs={!home} />}
        sidebar={false}
        footer={false}
        components={components}
      />
      <MonoFooter product="ferrigura" home="/">
        <a href="https://github.com/sebastian-software/ferrigura">GitHub</a>
        <a href="https://crates.io/crates/ferrigura">crates.io</a>
        <a href="https://www.npmjs.com/package/@ferrigura/core">npm</a>
        <a href="/notices/diagram-fonts.txt">Diagram font licenses</a>
        <a href="https://ardo-docs.dev">Built with Ardo</a>
      </MonoFooter>
    </>
  );
  if (home) return <MonoPage className="fgr-site">{shell}</MonoPage>;
  return (
    <MonoPage className="mono-docs fgr-docs">
      <MarkDefs />
      {shell}
    </MonoPage>
  );
}
