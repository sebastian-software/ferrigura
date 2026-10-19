//! The domains of the diagram and where they are drawn: `getDomainLayouts` (`cynefinRenderer.ts`
//! 40-50), the names, the texts and the classes of each domain (`DOMAIN_META`, 23-29, and the labels
//! of 168-189), and the constants of the item badges (229-231).

/// The height of an item badge (`itemHeight`, `cynefinRenderer.ts` 230).
pub(super) const ITEM_HEIGHT: f64 = 26.0;

/// The gap between two badges, so that they start 30 apart (`itemHeight + 4`, line 263).
pub(super) const ITEM_PITCH: f64 = ITEM_HEIGHT + 4.0;

/// The space on each side of an item's text in its badge (`itemPaddingX`, line 231).
pub(super) const ITEM_PADDING_X: f64 = 10.0;

/// The items the confusion ellipse shows before the `+N more` badge (`MAX_CONFUSION_ITEMS`, line 69).
pub(super) const MAX_CONFUSION_ITEMS: usize = 3;

/// A domain of the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Domain {
    /// Complex, the top left quadrant.
    Complex,
    /// Complicated, the top right quadrant.
    Complicated,
    /// Chaotic, the bottom left quadrant.
    Chaotic,
    /// Clear, the bottom right quadrant.
    Clear,
    /// Confusion, the ellipse at the centre.
    Confusion,
}

impl Domain {
    /// The quadrants, in the order the reference draws their backgrounds and their labels.
    pub(super) const QUADRANTS: [Self; 4] =
        [Self::Complex, Self::Complicated, Self::Chaotic, Self::Clear];

    /// The domains, in the order the items are drawn (`cynefinRenderer.ts` 233).
    pub(super) const ITEMS: [Self; 5] = [
        Self::Complex,
        Self::Complicated,
        Self::Chaotic,
        Self::Clear,
        Self::Confusion,
    ];

    /// The domain of a name in the model, or `None` for a name the framework does not have. The
    /// reference's `layouts[name]` is then undefined, and a transition to it is not drawn.
    #[must_use]
    pub(super) fn of(name: &str) -> Option<Self> {
        match name {
            "complex" => Some(Self::Complex),
            "complicated" => Some(Self::Complicated),
            "chaotic" => Some(Self::Chaotic),
            "clear" => Some(Self::Clear),
            "confusion" => Some(Self::Confusion),
            _ => None,
        }
    }

    /// The name of the domain, as the block and the transitions of the diagram write it.
    #[must_use]
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Complex => "complex",
            Self::Complicated => "complicated",
            Self::Chaotic => "chaotic",
            Self::Clear => "clear",
            Self::Confusion => "confusion",
        }
    }

    /// The label of the domain: its name with a capital (`charAt(0).toUpperCase()`), as drawn.
    #[must_use]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Complex => "Complex",
            Self::Complicated => "Complicated",
            Self::Chaotic => "Chaotic",
            Self::Clear => "Clear",
            Self::Confusion => "Confusion",
        }
    }

    /// The class that marks the elements filled with the domain's colour. The reference fills them
    /// with a presentation attribute only; ferrigura adds this class, so that the colour can also be a
    /// custom property that the colour scheme chooses (`docs-port/decisions/0007`). The geometry
    /// harness leaves the class out of the structural key, as it does the pie's slot classes.
    #[must_use]
    pub(super) fn fill_class(self) -> &'static str {
        match self {
            Self::Complex => "cynefin-fill-complex",
            Self::Complicated => "cynefin-fill-complicated",
            Self::Chaotic => "cynefin-fill-chaotic",
            Self::Clear => "cynefin-fill-clear",
            Self::Confusion => "cynefin-fill-confusion",
        }
    }

    /// The decision model of a quadrant, as the subtitle above its name (`DOMAIN_META`, lines 24-27).
    /// The confusion domain has none.
    #[must_use]
    pub(super) fn model(self) -> &'static str {
        match self {
            Self::Complex => "Probe \u{2192} Sense \u{2192} Respond",
            Self::Complicated => "Sense \u{2192} Analyse \u{2192} Respond",
            Self::Clear => "Sense \u{2192} Categorise \u{2192} Respond",
            Self::Chaotic => "Act \u{2192} Sense \u{2192} Respond",
            Self::Confusion => "",
        }
    }

    /// The practice of the domain, the subtitle under its model (`DOMAIN_META`).
    #[must_use]
    pub(super) fn practice(self) -> &'static str {
        match self {
            Self::Complex => "Emergent Practices",
            Self::Complicated => "Good Practices",
            Self::Clear => "Best Practices",
            Self::Chaotic => "Novel Practices",
            Self::Confusion => "Disorder",
        }
    }
}

/// The place of a domain: the centre where its label and items are drawn, and the box of its
/// background (`getDomainLayouts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Place {
    /// The centre's x.
    pub cx: f64,
    /// The centre's y.
    pub cy: f64,
    /// The left edge of the background box.
    pub x: f64,
    /// The top edge of the background box.
    pub y: f64,
    /// The width of the background box.
    pub w: f64,
    /// The height of the background box.
    pub h: f64,
}

/// The place of a domain in a diagram of the given width and height (`getDomainLayouts`,
/// `cynefinRenderer.ts` 40-50). The confusion ellipse has its own centre, the middle of the diagram.
#[must_use]
pub(super) fn place(domain: Domain, width: f64, height: f64) -> Place {
    let hw = width / 2.0;
    let hh = height / 2.0;
    match domain {
        Domain::Complex => Place {
            cx: hw / 2.0,
            cy: hh / 2.0,
            x: 0.0,
            y: 0.0,
            w: hw,
            h: hh,
        },
        Domain::Complicated => Place {
            cx: hw + hw / 2.0,
            cy: hh / 2.0,
            x: hw,
            y: 0.0,
            w: hw,
            h: hh,
        },
        Domain::Chaotic => Place {
            cx: hw / 2.0,
            cy: hh + hh / 2.0,
            x: 0.0,
            y: hh,
            w: hw,
            h: hh,
        },
        Domain::Clear => Place {
            cx: hw + hw / 2.0,
            cy: hh + hh / 2.0,
            x: hw,
            y: hh,
            w: hw,
            h: hh,
        },
        Domain::Confusion => Place {
            cx: hw,
            cy: hh,
            x: hw * 0.7,
            y: hh * 0.7,
            w: hw * 0.6,
            h: hh * 0.6,
        },
    }
}
