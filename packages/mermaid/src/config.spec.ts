import * as configApi from './config.js';
import type { MermaidConfig } from './config.type.js';

describe('when working with site config', () => {
  beforeEach(() => {
    // Resets the site config to default config
    configApi.reset();
    configApi.setSiteConfig({});
  });
  it('should set site config and config properly', () => {
    const config_0 = { fontFamily: 'foo-font', fontSize: 150 };
    configApi.setSiteConfig(config_0);
    const config_1 = configApi.getSiteConfig();
    const config_2 = configApi.getConfig();
    expect(config_1.fontFamily).toEqual(config_0.fontFamily);
    expect(config_1.fontSize).toEqual(config_0.fontSize);
    expect(config_1).toEqual(config_2);
  });
  it('should respect secure keys when applying directives', () => {
    const config_0: MermaidConfig = {
      fontFamily: 'foo-font',
      securityLevel: 'strict', // can't be changed
      fontSize: 12345, // can't be changed
      secure: [...configApi.defaultConfig.secure!, 'fontSize'],
    };
    configApi.setSiteConfig(config_0);
    const directive: MermaidConfig = {
      fontFamily: 'baf',
      // fontSize and securityLevel shouldn't be changed
      fontSize: 54321,
      securityLevel: 'loose',
    };
    configApi.addDirective(directive);
    const cfg: MermaidConfig = configApi.getConfig();
    expect(cfg.fontFamily).toEqual(directive.fontFamily);
    expect(cfg.fontSize).toBe(config_0.fontSize);
    expect(cfg.securityLevel).toBe(config_0.securityLevel);
  });
  it('should allow setting partial options', () => {
    const defaultConfig = configApi.getConfig();

    configApi.setConfig({
      quadrantChart: {
        chartHeight: 600,
      },
    });

    const updatedConfig = configApi.getConfig();

    // deep options we didn't update should remain the same
    expect(defaultConfig.quadrantChart!.chartWidth).toEqual(
      updatedConfig.quadrantChart!.chartWidth
    );
  });
  it('should default swimlane layering options to true', () => {
    const config = configApi.getConfig();

    expect(config.swimlane?.ignoreCrossLaneEdges).toBe(true);
    expect(config.swimlane?.optimizeRanksByCrossings).toBe(true);
    expect(config.swimlane?.automaticLaneOrdering).toBe(false);
  });

  it('leaves ELK placement and alignment to the selected preset', () => {
    const config = configApi.getConfig();

    expect(config.elk?.nodePlacementStrategy).toBeUndefined();
    expect(config.elk?.nodePlacementAlignment).toBeUndefined();
  });

  it('keeps the flowchart inheritDir default in the flowchart section', () => {
    const config = configApi.getConfig();

    expect(config.flowchart?.inheritDir).toBe(false);
    expect(config.c4).not.toHaveProperty('flowchart');
  });

  it('should not force class diagram padding', () => {
    const config = configApi.getConfig();

    expect(config.class?.padding).toBeUndefined();
  });

  it('should retain railroad directives after sanitization', () => {
    configApi.saveConfigFromInitialize({});
    const railroad = {
      fontSize: 18,
      fontFamily: 'Courier New',
      // Options only railroad has, so `configKeys` knows them only from its schema defaults
      arcRadius: 4,
      compactMode: true,
      verticalSeparation: 12,
      horizontalSeparation: 16,
      showMarkers: false,
      markerRadius: 3,
    };
    // A copy, since sanitizeDirective deletes unknown keys in place
    configApi.addDirective({ railroad: { ...railroad } });

    expect(configApi.getConfig().railroad).toMatchObject(railroad);
  });
  it('should set reset config properly', () => {
    const config_0 = { fontFamily: 'foo-font', fontSize: 150 };
    configApi.setSiteConfig(config_0);
    const config_1 = { fontFamily: 'baf' };
    configApi.setConfig(config_1);
    const config_2 = configApi.getConfig();
    expect(config_2.fontFamily).toEqual(config_1.fontFamily);
    configApi.reset();
    const config_3 = configApi.getConfig();
    expect(config_3.fontFamily).toEqual(config_0.fontFamily);
    const config_4 = configApi.getSiteConfig();
    expect(config_4.fontFamily).toEqual(config_0.fontFamily);
  });
  it('should set global reset config properly', () => {
    const config_0 = { fontFamily: 'foo-font', fontSize: 150 };
    configApi.setSiteConfig(config_0);
    const config_1 = configApi.getSiteConfig();
    expect(config_1.fontFamily).toEqual(config_0.fontFamily);
    const config_2 = configApi.getConfig();
    expect(config_2.fontFamily).toEqual(config_0.fontFamily);
    configApi.setConfig({ darkMode: true });
    const config_3 = configApi.getConfig();
    expect(config_3.darkMode).toEqual(true);
    configApi.reset();
    const config_4 = configApi.getConfig();
    expect(config_4.darkMode).toEqual(false);
  });
});

describe('getUserDefinedConfig', () => {
  beforeEach(() => {
    configApi.reset();
  });

  it('should return empty object when no user config is defined', () => {
    const userConfig = configApi.getUserDefinedConfig();
    expect(userConfig).toEqual({});
  });

  it('should return config from initialize only', () => {
    const initConfig: MermaidConfig = { theme: 'dark', fontFamily: 'Arial' };
    configApi.saveConfigFromInitialize(initConfig);

    const userConfig = configApi.getUserDefinedConfig();
    expect(userConfig).toEqual(initConfig);
  });

  it('should return config from directives only', () => {
    const directive1: MermaidConfig = { layout: 'elk', fontSize: 14 };
    const directive2: MermaidConfig = { theme: 'forest' };

    configApi.addDirective(directive1);
    configApi.addDirective(directive2);

    expect(configApi.getUserDefinedConfig()).toMatchInlineSnapshot(`
      {
        "fontFamily": "Arial",
        "fontSize": 14,
        "layout": "elk",
        "theme": "forest",
      }
    `);
  });

  it('should combine initialize config and directives', () => {
    const initConfig: MermaidConfig = { theme: 'dark', fontFamily: 'Arial', layout: 'elk.stress' };
    const directive1: MermaidConfig = { layout: 'elk', fontSize: 14 };
    const directive2: MermaidConfig = { theme: 'forest' };

    configApi.saveConfigFromInitialize(initConfig);
    configApi.addDirective(directive1);
    configApi.addDirective(directive2);

    const userConfig = configApi.getUserDefinedConfig();
    expect(userConfig).toMatchInlineSnapshot(`
      {
        "fontFamily": "Arial",
        "fontSize": 14,
        "layout": "elk",
        "theme": "forest",
      }
    `);
  });

  it('should handle nested config objects properly', () => {
    const initConfig: MermaidConfig = {
      flowchart: { nodeSpacing: 50, rankSpacing: 100 },
      theme: 'default',
    };
    const directive: MermaidConfig = {
      flowchart: { nodeSpacing: 75, curve: 'basis' },
      mindmap: { padding: 20 },
    };

    configApi.saveConfigFromInitialize(initConfig);
    configApi.addDirective(directive);

    const userConfig = configApi.getUserDefinedConfig();
    expect(userConfig).toMatchInlineSnapshot(`
      {
        "flowchart": {
          "curve": "basis",
          "nodeSpacing": 75,
          "rankSpacing": 100,
        },
        "mindmap": {
          "padding": 20,
        },
        "theme": "default",
      }
    `);
  });

  it('should handle complex nested overrides', () => {
    const initConfig: MermaidConfig = {
      flowchart: {
        nodeSpacing: 50,
        rankSpacing: 100,
        curve: 'linear',
      },
      theme: 'default',
    };
    const directive1: MermaidConfig = {
      flowchart: {
        nodeSpacing: 75,
      },
      fontSize: 12,
    };
    const directive2: MermaidConfig = {
      flowchart: {
        curve: 'basis',
        nodeSpacing: 100,
      },
      mindmap: {
        padding: 15,
      },
    };

    configApi.saveConfigFromInitialize(initConfig);
    configApi.addDirective(directive1);
    configApi.addDirective(directive2);

    const userConfig = configApi.getUserDefinedConfig();
    expect(userConfig).toMatchInlineSnapshot(`
      {
        "flowchart": {
          "curve": "basis",
          "nodeSpacing": 100,
          "rankSpacing": 100,
        },
        "fontSize": 12,
        "mindmap": {
          "padding": 15,
        },
        "theme": "default",
      }
    `);
  });

  it('should return independent copies (not references)', () => {
    const initConfig: MermaidConfig = { theme: 'dark', flowchart: { nodeSpacing: 50 } };
    configApi.saveConfigFromInitialize(initConfig);

    const userConfig1 = configApi.getUserDefinedConfig();
    const userConfig2 = configApi.getUserDefinedConfig();

    userConfig1.theme = 'neutral';
    userConfig1.flowchart!.nodeSpacing = 999;

    expect(userConfig2).toMatchInlineSnapshot(`
      {
        "flowchart": {
          "nodeSpacing": 50,
        },
        "theme": "dark",
      }
    `);
  });

  it('should handle edge cases with undefined values', () => {
    const initConfig: MermaidConfig = { theme: 'dark', layout: undefined };
    const directive: MermaidConfig = { fontSize: 14, fontFamily: undefined };

    configApi.saveConfigFromInitialize(initConfig);
    configApi.addDirective(directive);

    expect(configApi.getUserDefinedConfig()).toMatchInlineSnapshot(`
      {
        "fontSize": 14,
        "layout": undefined,
        "theme": "dark",
      }
    `);
  });

  it('should retain config from initialize after reset', () => {
    const initConfig: MermaidConfig = { theme: 'dark' };
    const directive: MermaidConfig = { layout: 'elk' };

    configApi.saveConfigFromInitialize(initConfig);
    configApi.addDirective(directive);

    expect(configApi.getUserDefinedConfig()).toMatchInlineSnapshot(`
      {
        "layout": "elk",
        "theme": "dark",
      }
    `);

    configApi.reset();
  });
});

describe('getEffectiveHtmlLabels', () => {
  beforeEach(() => {
    configApi.reset();
  });

  it('should return true when root-level htmlLabels is true', () => {
    configApi.setSiteConfig({ htmlLabels: true });
    const config = configApi.getConfig();
    expect(configApi.getEffectiveHtmlLabels(config)).toBe(true);
  });

  it('should return false when root-level htmlLabels is false', () => {
    configApi.setSiteConfig({ htmlLabels: false });
    const config = configApi.getConfig();
    expect(configApi.getEffectiveHtmlLabels(config)).toBe(false);
  });

  it('should default to true when htmlLabels is not set', () => {
    configApi.setSiteConfig({});
    const config = configApi.getConfig();
    expect(configApi.getEffectiveHtmlLabels(config)).toBe(true);
  });

  it('should let a directive override the site config', () => {
    configApi.setSiteConfig({ htmlLabels: true });
    configApi.addDirective({ htmlLabels: false });
    const config = configApi.getConfig();
    expect(configApi.getEffectiveHtmlLabels(config)).toBe(false);
  });
});
