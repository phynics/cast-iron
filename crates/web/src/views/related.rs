//! The relationship panels of an entry screen: racial traits of a race, archetypes of a class,
//! the prerequisites, chain, and followers of a feat, spell lists of a spell, and so on.
//!
//! Every panel reads the small index files (`relations.json`, `lists.json`) and the catalog. No
//! panel opens a shard. A library of schema 1 or 2 has neither file; its entries then get no
//! panel, and nothing around them breaks.

use crate::fmt;
use crate::graph::{
    Relations, demote_headings, is_alternate_racial_trait, level_counts, list_of_class_entry,
    list_total, requirements_blocks,
};
use crate::library::Library;
use crate::route::Route;
use crate::views::chains::chain_tree;
use crate::views::icons;
use crate::views::rich::{blocks, plain_text};
use crate::views::search::facet_label;
use crate::views::sources::source_line;
use crate::views::spells::spells_noun;
use crate::views::util::{local_resource, sort_key};
use cast_iron_schema::{
    Block, EntryId, EntryKind, OptionGroupIndex, Relation, RelationKind, ReplacesIndex, RuleEntry,
    SpellListClass, SpellLists, TypedFields,
};
use leptos::prelude::*;
use std::rc::Rc;

/// The index files that the panels of a kind need, so that the entry screen loads only those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Needs {
    pub relations: bool,
    pub spell_lists: bool,
    pub option_groups: bool,
    pub replaces: bool,
}

pub fn needs(kind: EntryKind) -> Needs {
    match kind {
        EntryKind::Race | EntryKind::Feat => Needs {
            relations: true,
            spell_lists: false,
            option_groups: false,
            replaces: false,
        },
        EntryKind::Class => Needs {
            relations: true,
            spell_lists: true,
            option_groups: true,
            replaces: true,
        },
        EntryKind::Spell => Needs {
            relations: true,
            spell_lists: true,
            option_groups: false,
            replaces: false,
        },
        // The metadata line of an option names its group and, when the group has one class,
        // links that class. The relation index carries any third-party `changes` add-on.
        EntryKind::ClassOption => Needs {
            relations: true,
            spell_lists: false,
            option_groups: true,
            replaces: false,
        },
        // A third-party `changes` add-on can attach to a rule or an item too, so those pages
        // load the relation index as a feat, class or spell page does.
        EntryKind::Rule | EntryKind::Item => Needs {
            relations: true,
            spell_lists: false,
            option_groups: false,
            replaces: false,
        },
        EntryKind::Creature
        | EntryKind::PrestigeClass
        | EntryKind::Skill
        | EntryKind::Affliction
        | EntryKind::Trap
        | EntryKind::Trait
        | EntryKind::RacialTrait
        | EntryKind::Archetype => Needs::default(),
    }
}

/// The index data that was loaded for an entry. A field is `None` when the library lacks the file.
#[derive(Clone, Default)]
pub struct Indexes {
    /// The relation graph projected for discovery: both ends of every edge are active.
    pub relations: Option<Rc<Relations>>,
    /// The unfiltered graph. An add-on row keeps its label while its base's book is inactive,
    /// and a base page shows an add-on by the add-on's own book alone.
    pub all_relations: Option<Rc<Relations>>,
    pub spell_lists: Option<Rc<SpellLists>>,
    /// The class-option group vocabulary, for the class page's jump list and an option's
    /// metadata line.
    pub option_groups: Option<Rc<OptionGroupIndex>>,
    /// The printed replacements of archetypes and class options.
    pub replaces: Option<Rc<ReplacesIndex>>,
}

/// Items grouped under a heading, headings in name order and items in name order. Used to group
/// archetypes and alternate racial traits by the book they come from.
pub fn group_sorted(
    items: Vec<(EntryId, String, String)>,
) -> Vec<(String, Vec<(EntryId, String)>)> {
    let mut groups: Vec<(String, Vec<(EntryId, String)>)> = Vec::new();
    for (id, title, group) in items {
        match groups.iter_mut().find(|(g, _)| *g == group) {
            Some((_, list)) => list.push((id, title)),
            None => groups.push((group, vec![(id, title)])),
        }
    }
    groups.sort_by_key(|(g, _)| sort_key(g));
    for (_, list) in &mut groups {
        list.sort_by_key(|(id, title)| (sort_key(title), id.clone()));
    }
    groups
}

/// Resolved prerequisite feats of a feat entry with their either-or marks. The entry's own
/// relations come first; the flat field fills in when a library has only that.
pub fn prerequisite_links(entry: &RuleEntry) -> Vec<(EntryId, bool)> {
    let from_relations: Vec<(EntryId, bool)> = entry
        .relations
        .iter()
        .filter(|r| r.kind == RelationKind::Prerequisite)
        .map(|r: &Relation| (r.target.clone(), r.label.as_deref() == Some("or")))
        .collect();
    if !from_relations.is_empty() {
        return from_relations;
    }
    match &entry.fields {
        TypedFields::Feat(f) => f
            .prerequisite_feats
            .iter()
            .map(|id| (id.clone(), false))
            .collect(),
        _ => Vec::new(),
    }
}

fn title_of(library: &Library, id: &EntryId) -> String {
    library
        .stub(id)
        .map_or_else(|| id.to_string(), |s| s.title.clone())
}

fn source_title(library: &Library, id: &EntryId) -> String {
    library
        .stub(id)
        .and_then(|s| library.source(&s.source))
        .map_or_else(|| "Other".to_string(), |s| s.title.clone())
}

fn link(library: &Library, id: &EntryId) -> impl IntoView + use<> {
    view! { <a href=Route::entry(id.clone()).href()>{title_of(library, id)}</a> }
}

/// A group of links that opens by default when it is short.
fn group_view(heading: String, items: Vec<(EntryId, String)>, open_up_to: usize) -> impl IntoView {
    group_view_meta(heading, items, open_up_to, None)
}

/// A group of links, each with an optional muted line such as an archetype's `Replaces …`.
fn group_view_meta(
    heading: String,
    items: Vec<(EntryId, String)>,
    open_up_to: usize,
    meta: Option<&std::collections::BTreeMap<EntryId, String>>,
) -> impl IntoView {
    let n = items.len();
    let rows = items
        .into_iter()
        .map(|(id, title)| {
            let detail = meta.and_then(|meta| meta.get(&id)).cloned();
            let href = Route::entry(id).href();
            view! {
                <li>
                    <a href=href>{title}</a>
                    {detail.map(|detail| view! { <span class="muted rel-meta">{detail}</span> })}
                </li>
            }
        })
        .collect_view();
    view! {
        <details class="rel-group" open=n <= open_up_to>
            <summary>
                {icons::chevron_right("disclosure-chevron")}
                <span class="rel-group-name">{heading}</span>
                <span class="rel-count">{fmt::count(n)}</span>
            </summary>
            <ul class="rel-chips">{rows}</ul>
        </details>
    }
}

/// Level links of one list, as in the spell hub.
fn level_pills(list: &SpellListClass) -> impl IntoView + use<> {
    let name = list.name.clone();
    let items = level_counts(list)
        .into_iter()
        .map(|(level, n)| {
            let href = Route::SpellList {
                class: name.clone(),
                level: Some(level),
            }
            .href();
            let label = format!(
                "{name} {}, {}",
                fmt::spell_level(level).to_lowercase(),
                spells_noun(n)
            );
            let shown = if level == 0 {
                fmt::spell_level(level)
            } else {
                level.to_string()
            };
            view! {
                <li>
                    <a class="lvl" href=href aria-label=label>
                        <span class="lvl-n">{shown}</span>
                        <span class="lvl-count">{fmt::count(n)}</span>
                    </a>
                </li>
            }
        })
        .collect_view();
    view! { <ol class="lvls">{items}</ol> }
}

fn race_panel(library: &Library, id: &EntryId, relations: &Relations) -> Option<AnyView> {
    let parts = relations.sources_of(id, RelationKind::Part);
    if parts.is_empty() {
        return None;
    }
    let (alternate, standard): (Vec<&EntryId>, Vec<&EntryId>) = parts
        .into_iter()
        .partition(|p| is_alternate_racial_trait(p));
    let total = standard.len() + alternate.len();
    let standard = group_sorted(
        standard
            .into_iter()
            .map(|p| {
                (
                    p.clone(),
                    title_of(library, p),
                    "Standard racial traits".to_string(),
                )
            })
            .collect(),
    );
    let alternate = group_sorted(
        alternate
            .into_iter()
            .map(|p| {
                (
                    p.clone(),
                    title_of(library, p),
                    format!("Alternate racial traits, {}", source_title(library, p)),
                )
            })
            .collect(),
    );
    let groups = standard
        .into_iter()
        .chain(alternate)
        .map(|(heading, items)| group_view(heading, items, 16))
        .collect_view();
    Some(
        view! {
            <section class="rel" aria-labelledby="rel-traits">
                <h2 id="rel-traits">"Racial traits "<span class="rel-count">{fmt::count(total)}</span></h2>
                {groups}
            </section>
        }
        .into_any(),
    )
}

fn racial_trait_panel(library: &Library, entry: &RuleEntry) -> Option<AnyView> {
    let alternate = matches!(&entry.fields, TypedFields::RacialTrait(f) if f.alternate);
    let races: Vec<&Relation> = entry
        .relations
        .iter()
        .filter(|r| r.kind == RelationKind::Part && library.is_entry_active(&r.target))
        .collect();
    if races.is_empty() {
        return None;
    }
    let rows = races
        .iter()
        .map(|r| {
            let race_link = link(library, &r.target);
            view! { <li>{race_link}</li> }
        })
        .collect_view();
    let kind = if alternate {
        "An alternate racial trait of"
    } else {
        "A standard racial trait of"
    };
    Some(
        view! {
            <section class="rel" aria-labelledby="rel-part">
                <h2 id="rel-part">{kind}</h2>
                <ul class="rel-list">{rows}</ul>
                <p class="hint">"Open the race to see its other traits."</p>
            </section>
        }
        .into_any(),
    )
}

fn class_panels(
    library: &Library,
    id: &EntryId,
    relations: Option<&Relations>,
    lists: Option<&SpellLists>,
    option_groups: Option<&OptionGroupIndex>,
    replaces: Option<&ReplacesIndex>,
) -> Vec<AnyView> {
    let mut out = Vec::new();
    if let Some(list) = lists.and_then(|l| list_of_class_entry(l, id)) {
        let href = Route::SpellList {
            class: list.name.clone(),
            level: None,
        }
        .href();
        out.push(
            view! {
                <section class="rel" aria-labelledby="rel-spells">
                    <h2 id="rel-spells">"Spell list"</h2>
                    <p>
                        <a href=href>{format!("{} spell list", list.name)}</a>
                        <span class="muted">{format!(" · {}", spells_noun(list_total(list)))}</span>
                    </p>
                    {level_pills(list)}
                </section>
            }
            .into_any(),
        );
    }
    // Class options: one navigation rectangle per group this class takes, with the number of
    // options the active books keep. A shared group carries the class scope in the URL, so the
    // list page can name the class it came from.
    if let Some(index) = option_groups {
        let discovery = library.discovery();
        let groups = crate::class_options::groups_for_class(index, id, |option| {
            discovery.is_eligible(option.as_str())
        });
        if !groups.is_empty() {
            let rows = groups
                .into_iter()
                .map(|(group, count)| {
                    let mut facets = vec![("group".to_string(), group.slug.clone())];
                    if crate::class_options::serves_several_classes(group) {
                        facets.push(("class_scope".to_string(), id.to_string()));
                    }
                    let href = Route::browse_kind_facets(EntryKind::ClassOption, facets).href();
                    view! {
                        <li>
                            <a class="jump" href=href>
                                {group.label.clone()}
                                <span class="jump-count">{fmt::count(count)}</span>
                            </a>
                        </li>
                    }
                })
                .collect_view();
            out.push(
                view! {
                    <section class="rel" aria-labelledby="rel-class-options">
                        <h2 id="rel-class-options">"Class options"</h2>
                        <ul class="jump-list">{rows}</ul>
                    </section>
                }
                .into_any(),
            );
        }
    }
    if let Some(relations) = relations {
        let archetypes = relations.sources_of(id, RelationKind::Archetype);
        if !archetypes.is_empty() {
            let total = archetypes.len();
            let replaces_by_id: std::collections::BTreeMap<EntryId, String> = replaces
                .map(|index| {
                    index
                        .entries
                        .iter()
                        .filter(|row| !row.replaces.is_empty())
                        .map(|row| {
                            (
                                row.id.clone(),
                                format!("Replaces {}", row.replaces.join(", ")),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let groups = group_sorted(
                archetypes
                    .into_iter()
                    .map(|a| (a.clone(), title_of(library, a), source_title(library, a)))
                    .collect(),
            )
            .into_iter()
            .map(|(heading, items)| group_view_meta(heading, items, 16, Some(&replaces_by_id)))
            .collect_view();
            out.push(
                view! {
                    <section class="rel" aria-labelledby="rel-archetypes">
                        <h2 id="rel-archetypes">"Archetypes · "<span class="rel-count">{fmt::count(total)}</span></h2>
                        <details class="rel-disclosure">
                            <summary>{icons::chevron_right("disclosure-chevron")}{format!("Show {}", fmt::pluralize(total, "archetype", "archetypes"))}</summary>
                            {groups}
                        </details>
                    </section>
                }
                .into_any(),
            );
        }
    }
    out
}

fn archetype_panel(library: &Library, entry: &RuleEntry) -> Option<AnyView> {
    let classes: Vec<&Relation> = entry
        .relations
        .iter()
        .filter(|r| r.kind == RelationKind::Archetype)
        .filter(|r| library.is_entry_active(&r.target))
        .collect();
    if classes.is_empty() {
        return None;
    }
    let rows = classes
        .iter()
        .map(|r| {
            let class_link = link(library, &r.target);
            view! { <li>{class_link}</li> }
        })
        .collect_view();
    Some(
        view! {
            <section class="rel" aria-labelledby="rel-archetype-of">
                <h2 id="rel-archetype-of">"An archetype of"</h2>
                <ul class="rel-list">{rows}</ul>
                <p class="hint">"Open the class to see its other archetypes."</p>
            </section>
        }
        .into_any(),
    )
}

/// The add-ons that attach to `base` with `label` and pass `eligible`, in index order. The
/// edge's label picks the kind; the add-on's own entry decides eligibility, so the base's book
/// never hides an add-on.
pub fn addons_of_label(
    relations: &Relations,
    base: &EntryId,
    label: &str,
    eligible: impl Fn(&EntryId) -> bool,
) -> Vec<EntryId> {
    relations
        .addons_of(base, label)
        .into_iter()
        .filter(|id| eligible(id))
        .cloned()
        .collect()
}

/// The direct variants of `base` whose own book is active, in index order. A variant of a
/// variant names its direct base only, so a chain is walked one page at a time rather than
/// flattened here.
pub fn variants_of(
    relations: &Relations,
    base: &EntryId,
    eligible: impl Fn(&EntryId) -> bool,
) -> Vec<EntryId> {
    relations
        .sources_of(base, RelationKind::Variant)
        .into_iter()
        .filter(|id| eligible(id))
        .cloned()
        .collect()
}

/// One spell's class and level rows from the spell-list index, sorted as the page shows them:
/// class name, then level. Empty when the index does not list the spell. The index is already
/// in memory, so a variants row never opens the spell's shard.
fn spell_level_rows(lists: &SpellLists, id: &EntryId) -> Vec<(String, u8)> {
    let Some(position) = lists.spells.iter().position(|spell| &spell.0 == id) else {
        return Vec::new();
    };
    let position = position as u32;
    let mut rows = Vec::new();
    for class in &lists.classes {
        for level in &class.levels {
            if level.spells.contains(&position) {
                rows.push((class.name.clone(), level.level));
            }
        }
    }
    rows.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then(a.1.cmp(&b.1))
    });
    rows
}

/// The muted levels line of a variants row: `Bard 6, Cleric 5, Druid 6`, with level 0 reading
/// as `Cantrips` through the shared formatter. `None` when the index lists no class for the
/// spell, so the row falls back to its book alone.
fn levels_line(rows: &[(String, u8)]) -> Option<String> {
    if rows.is_empty() {
        return None;
    }
    Some(
        rows.iter()
            .map(|(class, level)| {
                if *level == 0 {
                    format!("{class} {}", fmt::spell_level(0))
                } else {
                    format!("{class} {level}")
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// The add-on's body as the base page shows it: no stat block, because the entry's own stat
/// block keeps the page's only accent rule, and every heading as a bold paragraph, so the
/// section's `h2` and the add-on's `h3` title stay its only headings.
pub fn attached_body(body: &[Block]) -> Vec<Block> {
    body.iter()
        .filter(|block| !matches!(block, Block::StatBlock { .. }))
        .map(demote_headings)
        .collect()
}

/// The shard-backed rules text of one attached add-on: a status line while it loads, the
/// error if the shard fails, then the body that [`attached_body`] prepared. The add-on's own
/// citations follow the body. `sources_fallback` names the book for an add-on that never had a
/// printed citation (a mythic version), so the section still says where the text comes from.
#[component]
fn AddonBody(
    library: Library,
    id: EntryId,
    loading: &'static str,
    #[prop(optional)] sources_fallback: Option<String>,
) -> impl IntoView {
    let load = library.clone();
    let body = local_resource(async move { load.entry(&id).await });
    view! {
        {move || {
            body.with(|state| match state {
                None => view! { <p class="status" role="status">{loading}</p> }.into_any(),
                Some(Err(error)) => view! {
                    <p class="error" role="alert">{error.to_string()}</p>
                }.into_any(),
                Some(Ok(entry)) => {
                    let sources = source_line(
                        &entry.sources,
                        &library,
                        sources_fallback.as_deref(),
                    );
                    view! {
                        <div class="rich">{blocks(&attached_body(&entry.body))}</div>
                        {sources}
                    }.into_any()
                },
            })
        }}
    }
}

/// One mythic version inside a base page's `Mythic` section. The title arrives with the page;
/// the add-on's typed citations follow its text, and an unconverted version falls back to its
/// book name because its body has no printed line.
#[component]
fn MythicAddon(library: Library, id: EntryId) -> impl IntoView {
    let href = Route::entry(id.clone()).href();
    let title = title_of(&library, &id);
    let book = source_title(&library, &id);
    view! {
        <div class="attached-addon">
            <h3><a href=href>{title}</a></h3>
            <AddonBody library=library id=id loading="Loading the mythic rules…" sources_fallback=book />
        </div>
    }
}

fn mythic_panel(library: &Library, id: &EntryId, all: Option<&Relations>) -> Option<AnyView> {
    let relations = all?;
    let discovery = library.discovery();
    let addons = addons_of_label(relations, id, "mythic", |addon| {
        discovery.is_eligible(addon.as_str())
    });
    if addons.is_empty() {
        return None;
    }
    let versions = addons
        .into_iter()
        .map(|addon| view! { <MythicAddon library=library.clone() id=addon /> }.into_any())
        .collect_view();
    Some(
        view! {
            <section class="rel-addon" aria-labelledby="rel-mythic">
                <h2 id="rel-mythic">"Mythic"</h2>
                {versions}
            </section>
        }
        .into_any(),
    )
}

/// One collapsed Stamina disclosure on a base page. The summary names the add-on's book; the
/// shard-backed rules text is created on the first open and kept through close and reopen, so
/// a closed row reads nothing. No title, box or tint: the summary is the heading.
#[component]
fn StaminaAddon(library: Library, id: EntryId) -> impl IntoView {
    let book = source_title(&library, &id);
    let opened = RwSignal::new(false);
    let on_toggle = move |event: leptos::ev::Event| {
        if opened.get_untracked() {
            return;
        }
        opened.set(event_target::<web_sys::HtmlDetailsElement>(&event).open());
    };
    view! {
        <details class="rel-disclosure rel-stamina" on:toggle=on_toggle>
            <summary>{icons::chevron_right("disclosure-chevron")}{format!("Stamina · {book}")}</summary>
            {move || opened.get().then(|| view! {
                <AddonBody library=library.clone() id=id.clone() loading="Loading the Stamina rules…" />
            })}
        </details>
    }
}

/// The collapsed Stamina rows a base page shows, one per eligible add-on, after the Mythic
/// section. The add-on's own page carries the same text; this is a second way to reach it.
fn stamina_panel(library: &Library, id: &EntryId, all: Option<&Relations>) -> Vec<AnyView> {
    let Some(relations) = all else {
        return Vec::new();
    };
    let discovery = library.discovery();
    addons_of_label(relations, id, "stamina", |addon| {
        discovery.is_eligible(addon.as_str())
    })
    .into_iter()
    .map(|addon| view! { <StaminaAddon library=library.clone() id=addon /> }.into_any())
    .collect()
}

/// One third-party change inside a base page's book-titled section: the add-on's title as an
/// `h3` link and its printed text, expanded like a mythic version. The add-on's typed citations
/// follow the body. An unconverted body keeps its printed `Sources:` paragraph, so the section
/// never adds a second line for it.
#[component]
fn ChangesAddon(library: Library, id: EntryId) -> impl IntoView {
    let href = Route::entry(id.clone()).href();
    let title = title_of(&library, &id);
    view! {
        <h3><a href=href>{title}</a></h3>
        <AddonBody library=library id=id loading="Loading the third-party changes…" />
    }
}

/// The expanded third-party changes a base page shows, one section per eligible add-on, its
/// heading naming the add-on's book. The add-on's own book alone gates the section; the base's
/// book never does. An inactive base still shows it, as it does for a mythic version.
fn changes_panel(library: &Library, id: &EntryId, all: Option<&Relations>) -> Vec<AnyView> {
    let Some(relations) = all else {
        return Vec::new();
    };
    let discovery = library.discovery();
    addons_of_label(relations, id, "changes", |addon| {
        discovery.is_eligible(addon.as_str())
    })
    .into_iter()
    .map(|addon| {
        let book = source_title(library, &addon);
        let heading = format!("Changes in {book}");
        let label = heading.clone();
        view! {
            <section class="rel-addon rel-changes" aria-label=label>
                <h2>{heading}</h2>
                <ChangesAddon library=library.clone() id=addon />
            </section>
        }
        .into_any()
    })
    .collect()
}

fn feat_panels(
    library: &Library,
    entry: &RuleEntry,
    relations: Option<&Relations>,
) -> Vec<AnyView> {
    let mut out = Vec::new();
    let text = match &entry.fields {
        TypedFields::Feat(f) => f.prerequisites.clone().filter(|t| !t.trim().is_empty()),
        _ => None,
    };
    let named: Vec<_> = prerequisite_links(entry)
        .into_iter()
        .filter(|(id, _)| library.is_entry_active(id))
        .collect();
    if text.is_some() || !named.is_empty() {
        let printed = text.map(|t| {
            view! {
                <p class="prereq-text"><span class="rel-label">"As printed: "</span>{t}</p>
            }
        });
        let named_view = (!named.is_empty()).then(|| {
            let items = named
                .iter()
                .map(|(id, alt)| {
                    let l = link(library, id);
                    let mark = alt.then(|| view! { <span class="tag">"or"</span> });
                    view! { <li>{l}{mark}</li> }
                })
                .collect_view();
            view! {
                <p class="rel-label">"Feats it names"</p>
                <ul class="rel-chips">{items}</ul>
            }
        });
        out.push(
            view! {
                <section class="rel rel-prereq" aria-labelledby="rel-prereq">
                    <h2 id="rel-prereq">"Requirements"</h2>
                    {printed}
                    {named_view}
                </section>
            }
            .into_any(),
        );
    }
    if let Some(relations) = relations {
        let chain = relations.prerequisite_chain(&entry.id);
        if !chain.next.is_empty() {
            let n = relations.ancestors(&entry.id).len();
            let tree = chain_tree(library, &chain, false);
            let title = entry.title.clone();
            out.push(
                view! {
                    <section class="rel" aria-labelledby="rel-chain">
                        <h2 id="rel-chain">"Prerequisite chain "<span class="rel-count">{fmt::count(n)}</span></h2>
                        <p class="hint">
                            {format!("Each feat is a prerequisite of the one above it. {title} is at the top.")}
                        </p>
                        <ul class="chain chain-root">
                            <li>
                                <span class="chain-self" aria-current="true">{entry.title.clone()}</span>
                                {tree}
                            </li>
                        </ul>
                    </section>
                }
                .into_any(),
            );
        }
        let mut leads: Vec<(EntryId, String)> = relations
            .leads_to(&entry.id)
            .into_iter()
            .map(|id| (id.clone(), title_of(library, id)))
            .collect();
        leads.sort_by_key(|(id, title)| (sort_key(title), id.clone()));
        if !leads.is_empty() {
            let n = leads.len();
            let rows = leads
                .into_iter()
                .map(|(id, title)| view! { <li><a href=Route::entry(id).href()>{title}</a></li> })
                .collect_view();
            out.push(
                view! {
                    <section class="rel" aria-labelledby="rel-leads">
                        <h2 id="rel-leads">"Leads to "<span class="rel-count">{fmt::count(n)}</span></h2>
                        <p class="hint">"Feats that name this feat in their prerequisites."</p>
                        <ul class="rel-list rel-cols">{rows}</ul>
                    </section>
                }
                .into_any(),
            );
        }
    }
    out
}

/// Resolve only names explicitly present in the Feats or Skills requirements section.
/// Duplicate catalog names remain text; no source is chosen on the reader's behalf.
fn requirement_links(
    body: &[Block],
    candidates: &[(EntryId, String, EntryKind)],
) -> Vec<(EntryId, String)> {
    fn normalized(text: &str) -> String {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn text(block: &Block) -> String {
        match block {
            Block::Paragraph { content, .. } => plain_text(content),
            Block::List { items, .. } => items
                .iter()
                .flatten()
                .map(text)
                .collect::<Vec<_>>()
                .join(" "),
            Block::Quote { content } => content.iter().map(text).collect::<Vec<_>>().join(" "),
            _ => String::new(),
        }
    }
    let mut requirement_level = None;
    let mut section = None;
    let mut feat_text = String::new();
    let mut skill_text = String::new();
    for block in body {
        if let Block::Heading { level, content, .. } = block {
            let heading = normalized(&plain_text(content));
            if heading.starts_with("requirements") {
                requirement_level = Some(*level);
                section = None;
                continue;
            }
            if let Some(root) = requirement_level {
                if *level <= root {
                    break;
                }
                section = match heading.as_str() {
                    "feats" => Some(EntryKind::Feat),
                    "skills" => Some(EntryKind::Skill),
                    _ => None,
                };
            }
        } else if requirement_level.is_some() {
            match section {
                Some(EntryKind::Feat) => {
                    feat_text.push(' ');
                    feat_text.push_str(&text(block));
                }
                Some(EntryKind::Skill) => {
                    skill_text.push(' ');
                    skill_text.push_str(&text(block));
                }
                _ => {}
            }
        }
    }
    let feat_text = format!(" {} ", normalized(&feat_text));
    let skill_text = format!(" {} ", normalized(&skill_text));
    let mut names = std::collections::BTreeMap::<String, Vec<&(EntryId, String, EntryKind)>>::new();
    for candidate in candidates {
        names
            .entry(normalized(&candidate.1))
            .or_default()
            .push(candidate);
    }
    let mut found = Vec::new();
    for (name, matches) in names {
        if name.is_empty() {
            continue;
        }
        for (id, title, kind) in matches {
            let haystack = match kind {
                EntryKind::Feat => &feat_text,
                EntryKind::Skill => &skill_text,
                _ => continue,
            };
            if let Some(position) = haystack.find(&format!(" {name} ")) {
                found.push((position, id.clone(), title.clone()));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    found
        .into_iter()
        .map(|(_, id, title)| (id, title))
        .collect()
}

fn prestige_panel(library: &Library, entry: &RuleEntry) -> Option<AnyView> {
    let requirements = requirements_blocks(&entry.body);
    if requirements.is_empty() {
        return None;
    }
    let candidates = library
        .discovery()
        .entries()
        .filter(|s| matches!(s.kind, EntryKind::Feat | EntryKind::Skill))
        .map(|s| (s.id.clone(), s.title.clone(), s.kind))
        .collect::<Vec<_>>();
    let links = requirement_links(&entry.body, &candidates);
    let has_links = !links.is_empty();
    let variants = links
        .into_iter()
        .filter_map(|(id, title)| {
            let stub = library.stub(&id)?;
            Some(crate::views::source_variants::SourceVariant {
                id,
                title,
                kind: stub.kind,
                source: library
                    .source(&stub.source)
                    .map_or_else(|| stub.source.clone(), |s| s.title.clone()),
            })
        })
        .collect();
    let links = crate::views::source_variants::group_variants(variants).into_iter().map(|group| {
        if group.variants.len() == 1 {
            let variant = &group.variants[0];
            view! { <li><a href=Route::entry(variant.id.clone()).href()>{variant.title.clone()}</a></li> }.into_any()
        } else {
            view! { <li><crate::views::source_variants::SourceVariants group /></li> }.into_any()
        }
    }).collect_view();
    Some(
        view! {
            <section class="rel rel-requirements" aria-labelledby="rel-requirements">
                <h2 id="rel-requirements">"Entry requirements, as stated"</h2>
                <div class="rich">{blocks(&requirements)}</div>
                {has_links.then(|| view! { <p class="rel-label">"Named feats and skills"</p><ul class="rel-chips">{links}</ul> })}
            </section>
        }
        .into_any(),
    )
}

fn trait_panel(entry: &RuleEntry) -> Option<AnyView> {
    let TypedFields::Trait(fields) = &entry.fields else {
        return None;
    };
    let route = Route::Search {
        q: String::new(),
        kinds: vec![EntryKind::Trait],
        sources: Vec::new(),
        facets: vec![("trait_category".into(), fields.category.to_lowercase())],
    };
    let detail = fields.detail.clone();
    Some(
        view! {
            <section class="rel" aria-labelledby="rel-trait">
                <h2 id="rel-trait">"Trait category"</h2>
                <p>
                    <a href=route.href()>{facet_label("trait_category", &fields.category)}</a>
                    {detail.map(|d| view! { <span class="muted">{format!(" · {d}")}</span> })}
                </p>
                <p class="hint">"The link lists every trait of this category."</p>
            </section>
        }
        .into_any(),
    )
}

/// The direct variants of a base spell, after the body. Each row links the variant and names
/// its levels and book from the indexes already in memory, so the panel opens no shard. The
/// variant's own book alone gates the row; the base's book never does.
fn variants_panel(
    library: &Library,
    id: &EntryId,
    all: Option<&Relations>,
    lists: Option<&SpellLists>,
) -> Option<AnyView> {
    let relations = all?;
    let discovery = library.discovery();
    let mut variants = variants_of(relations, id, |variant| {
        discovery.is_eligible(variant.as_str())
    })
    .into_iter()
    .map(|variant| (variant.clone(), title_of(library, &variant)))
    .collect::<Vec<_>>();
    if variants.is_empty() {
        return None;
    }
    variants.sort_by(|(a_id, a_title), (b_id, b_title)| {
        (sort_key(a_title), a_id.clone()).cmp(&(sort_key(b_title), b_id.clone()))
    });
    let rows = variants
        .into_iter()
        .map(|(variant, title)| {
            let href = Route::entry(variant.clone()).href();
            let book = source_title(library, &variant);
            let levels = lists
                .map(|lists| spell_level_rows(lists, &variant))
                .unwrap_or_default();
            let meta = match levels_line(&levels) {
                Some(levels) => format!("{levels} · {book}"),
                None => book,
            };
            view! {
                <li>
                    <a href=href>{title}</a>
                    <span class="muted rel-meta">{meta}</span>
                </li>
            }
        })
        .collect_view();
    Some(
        view! {
            <section class="rel-variants" aria-labelledby="rel-variants">
                <h2 id="rel-variants">"Variants"</h2>
                <ul class="rel-list">{rows}</ul>
            </section>
        }
        .into_any(),
    )
}

fn spell_panel(entry: &RuleEntry, lists: Option<&SpellLists>) -> Option<AnyView> {
    let TypedFields::Spell(fields) = &entry.fields else {
        return None;
    };
    if fields.levels.is_empty() {
        return None;
    }
    let mut levels = fields.levels.clone();
    if let Some(lists) = lists {
        levels.retain(|level| {
            crate::graph::find_list(lists, &level.class)
                .is_some_and(|list| list.levels.iter().any(|group| group.level == level.level))
        });
    }
    if levels.is_empty() {
        return None;
    }
    levels.sort_by(|a, b| {
        a.class
            .to_lowercase()
            .cmp(&b.class.to_lowercase())
            .then(a.level.cmp(&b.level))
    });
    let items = levels
        .into_iter()
        .map(|l| {
            let text = if l.level == 0 {
                format!("{} {}", l.class, fmt::spell_level(0))
            } else {
                format!("{} {}", l.class, l.level)
            };
            if lists.is_some() {
                let href = Route::SpellList {
                    class: l.class.clone(),
                    level: Some(l.level),
                }
                .href();
                view! { <li><a class="jump" href=href>{text}</a></li> }.into_any()
            } else {
                view! { <li><span class="tag">{text}</span></li> }.into_any()
            }
        })
        .collect_view();
    Some(
        view! {
            <section class="rel" aria-labelledby="rel-lists">
                <h2 id="rel-lists">"On spell lists"</h2>
                <ul class="jump-list">{items}</ul>
            </section>
        }
        .into_any(),
    )
}

/// The panels for one entry, in the order a reader needs them, or nothing.
#[component]
pub fn Related(library: Library, entry: RuleEntry, indexes: Indexes) -> impl IntoView {
    let relations = indexes.relations.as_deref();
    let all_relations = indexes.all_relations.as_deref();
    let lists = indexes.spell_lists.as_deref();
    let mut panels: Vec<AnyView> = Vec::new();
    match entry.kind {
        EntryKind::Race => {
            panels.extend(relations.and_then(|r| race_panel(&library, &entry.id, r)))
        }
        EntryKind::RacialTrait => panels.extend(racial_trait_panel(&library, &entry)),
        EntryKind::Class => {
            panels.extend(changes_panel(&library, &entry.id, all_relations));
            panels.extend(class_panels(
                &library,
                &entry.id,
                relations,
                lists,
                indexes.option_groups.as_deref(),
                indexes.replaces.as_deref(),
            ));
        }
        EntryKind::Archetype => panels.extend(archetype_panel(&library, &entry)),
        EntryKind::PrestigeClass => panels.extend(prestige_panel(&library, &entry)),
        EntryKind::Feat => {
            panels.extend(mythic_panel(&library, &entry.id, all_relations));
            panels.extend(changes_panel(&library, &entry.id, all_relations));
            panels.extend(stamina_panel(&library, &entry.id, all_relations));
            panels.extend(feat_panels(&library, &entry, relations));
        }
        EntryKind::Trait => panels.extend(trait_panel(&entry)),
        EntryKind::Spell => {
            panels.extend(mythic_panel(&library, &entry.id, all_relations));
            panels.extend(changes_panel(&library, &entry.id, all_relations));
            panels.extend(spell_panel(&entry, lists));
            panels.extend(variants_panel(&library, &entry.id, all_relations, lists));
        }
        // A third-party change can attach to a rule, an item or a class option. The panel
        // reads the raw graph, and the add-on's own book alone decides whether it appears.
        EntryKind::Rule | EntryKind::Item | EntryKind::ClassOption => {
            panels.extend(changes_panel(&library, &entry.id, all_relations));
        }
        EntryKind::Creature | EntryKind::Skill | EntryKind::Affliction | EntryKind::Trap => {}
    }
    (!panels.is_empty()).then(|| view! { <div class="entry-rel">{panels}</div> })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cast_iron_schema::{FeatFields, Provenance};
    fn id(s: &str) -> EntryId {
        EntryId::parse(s).unwrap()
    }

    #[test]
    fn requirements_resolve_punctuation_keep_variants_and_skip_other_sections() {
        let heading = |title: &str, level| Block::Heading {
            level,
            anchor: None,
            content: vec![cast_iron_schema::Inline::text(title)],
        };
        let para = |title: &str| Block::Paragraph {
            style: cast_iron_schema::ParagraphStyle::Normal,
            content: vec![cast_iron_schema::Inline::text(title)],
        };
        let body = vec![
            heading("Requirements", 2),
            heading("Feats", 3),
            para("Point Blank Shot, Precise Shot, Weapon Focus (longbow)."),
            heading("Spells", 3),
            para("Fly"),
            heading("Class Features", 2),
            para("Other"),
        ];
        let candidate = |id_text: &str, title: &str, kind| (id(id_text), title.to_string(), kind);
        let result = requirement_links(
            &body,
            &[
                candidate("c/point", "Point-Blank Shot", EntryKind::Feat),
                candidate("c/precise", "Precise Shot", EntryKind::Feat),
                candidate("u/precise", "Precise Shot", EntryKind::Feat),
                candidate("c/focus", "Weapon Focus", EntryKind::Feat),
                candidate("c/fly", "Fly", EntryKind::Skill),
                candidate("c/other", "Other", EntryKind::Feat),
            ],
        );
        assert_eq!(
            result
                .iter()
                .map(|(_, name)| name.as_str())
                .collect::<Vec<_>>(),
            [
                "Point-Blank Shot",
                "Precise Shot",
                "Precise Shot",
                "Weapon Focus"
            ]
        );
    }

    #[test]
    fn groups_come_in_name_order_and_keep_every_item() {
        let items = vec![
            (
                id("u/zealot"),
                "Zealot".to_string(),
                "Ultimate Combat".to_string(),
            ),
            (
                id("a/ace"),
                "Ace".to_string(),
                "Advanced Player's Guide".to_string(),
            ),
            (
                id("u/brawler"),
                "Brawler".to_string(),
                "Ultimate Combat".to_string(),
            ),
        ];
        let groups = group_sorted(items);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Advanced Player's Guide");
        let titles: Vec<&str> = groups[1].1.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(titles, ["Brawler", "Zealot"]);
    }

    fn feat(prereq_feats: &[&str], relations: Vec<Relation>) -> RuleEntry {
        RuleEntry {
            id: id("c/feats/x"),
            kind: EntryKind::Feat,
            title: "X".into(),
            aon: None,
            source_ids: vec!["c".into()],
            sources: vec![],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::Feat(FeatFields {
                prerequisite_feats: prereq_feats.iter().map(|s| id(s)).collect(),
                ..Default::default()
            }),
            tags: vec![],
            race_group: None,
            links: vec![],
            provenance: vec![Provenance {
                dataset: "d".into(),
                record: "r".into(),
                path: "p".into(),
            }],
            license_refs: vec![],
            relations,
        }
    }

    #[test]
    fn prerequisite_links_mark_either_or_and_fall_back_to_the_field() {
        let rel = |target: &str, label: Option<&str>| Relation {
            kind: RelationKind::Prerequisite,
            target: id(target),
            label: label.map(String::from),
        };
        let entry = feat(&[], vec![rel("c/a", Some("or")), rel("c/b", None)]);
        assert_eq!(
            prerequisite_links(&entry),
            [(id("c/a"), true), (id("c/b"), false)]
        );
        // A schema-3 field without relations still shows its feats.
        let flat = feat(&["c/z"], vec![]);
        assert_eq!(prerequisite_links(&flat), [(id("c/z"), false)]);
        assert!(prerequisite_links(&feat(&[], vec![])).is_empty());
    }

    #[test]
    fn only_kinds_with_panels_load_index_files() {
        assert!(needs(EntryKind::Feat).relations);
        assert!(needs(EntryKind::Class).spell_lists);
        // A third-party `changes` add-on can attach to a rule, an item or a class option.
        assert!(needs(EntryKind::Rule).relations);
        assert!(needs(EntryKind::Item).relations);
        assert!(needs(EntryKind::ClassOption).relations);
        assert_eq!(needs(EntryKind::Creature), Needs::default());
        // Racial traits, archetypes, and traits read their own entry: no extra file.
        assert_eq!(needs(EntryKind::Archetype), Needs::default());
    }

    fn labelled_edges(edges: &[(&str, &str, &str, Option<&str>)]) -> Relations {
        Relations::new(cast_iron_schema::RelationIndex {
            counts: std::collections::BTreeMap::new(),
            edges: edges
                .iter()
                .map(|(source, kind, target, label)| {
                    let kind = match *kind {
                        "addon" => RelationKind::Addon,
                        "variant" => RelationKind::Variant,
                        _ => RelationKind::Prerequisite,
                    };
                    cast_iron_schema::RelationEdge(
                        id(source),
                        kind,
                        id(target),
                        label.map(String::from),
                    )
                })
                .collect(),
        })
    }

    #[test]
    fn a_base_page_stacks_a_mythic_add_on_only_while_its_own_book_is_active() {
        let relations = labelled_edges(&[
            ("c/base/mythic", "addon", "c/base", Some("mythic")),
            ("c/other/stamina", "addon", "c/other", Some("stamina")),
        ]);
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "mythic", |_| true),
            [id("c/base/mythic")]
        );
        // The add-on's book is what gates the section; the base's book never does.
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "mythic", |addon| addon.as_str()
                != "c/base/mythic"),
            Vec::<EntryId>::new()
        );
        // A Stamina add-on is not a mythic version.
        assert!(addons_of_label(&relations, &id("c/other"), "mythic", |_| true).is_empty());
    }

    #[test]
    fn a_base_page_folds_a_stamina_add_on_only_while_its_own_book_is_active() {
        let relations = labelled_edges(&[
            ("c/base/mythic", "addon", "c/base", Some("mythic")),
            ("c/base/stamina", "addon", "c/base", Some("stamina")),
        ]);
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "stamina", |_| true),
            [id("c/base/stamina")]
        );
        // The add-on's own book gates the row; the base's book never does.
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "stamina", |addon| addon.as_str()
                != "c/base/stamina"),
            Vec::<EntryId>::new()
        );
        // A mythic version is not a Stamina row.
        assert!(
            addons_of_label(&relations, &id("c/base"), "stamina", |addon| addon.as_str()
                == "c/base/mythic")
            .is_empty()
        );
        // A base with no add-on gets no row.
        assert!(addons_of_label(&relations, &id("c/none"), "stamina", |_| true).is_empty());
    }

    #[test]
    fn a_base_page_shows_a_changes_add_on_only_while_its_own_book_is_active() {
        let relations = labelled_edges(&[
            ("c/base/teitr", "addon", "c/base", Some("changes")),
            ("c/base/stamina", "addon", "c/base", Some("stamina")),
        ]);
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "changes", |_| true),
            [id("c/base/teitr")]
        );
        // The add-on's own book gates the section; the base's book never does.
        assert_eq!(
            addons_of_label(&relations, &id("c/base"), "changes", |addon| addon.as_str()
                != "c/base/teitr"),
            Vec::<EntryId>::new()
        );
        // A Stamina add-on is not a third-party change.
        assert!(
            addons_of_label(&relations, &id("c/base"), "changes", |addon| addon.as_str()
                == "c/base/stamina")
            .is_empty()
        );
        // A base with no add-on gets no section.
        assert!(addons_of_label(&relations, &id("c/none"), "changes", |_| true).is_empty());
    }

    #[test]
    fn a_spell_base_lists_only_its_direct_variants_and_only_while_their_books_are_active() {
        let relations = labelled_edges(&[
            ("c/spell/base/mass", "variant", "c/spell/base", None),
            (
                "c/spell/base/mass/modified",
                "variant",
                "c/spell/base/mass",
                None,
            ),
            ("c/spell/other", "variant", "c/spell/other-base", None),
        ]);
        assert_eq!(
            variants_of(&relations, &id("c/spell/base"), |_| true),
            [id("c/spell/base/mass")]
        );
        // A chain is not flattened: the mass spell names its own direct variant only.
        assert_eq!(
            variants_of(&relations, &id("c/spell/base/mass"), |_| true),
            [id("c/spell/base/mass/modified")]
        );
        // The variant's own book gates the row; a missing base has no row either.
        assert!(variants_of(&relations, &id("c/spell/base"), |_| false).is_empty());
        assert!(variants_of(&relations, &id("c/spell/none"), |_| true).is_empty());
        assert!(
            variants_of(&relations, &id("c/spell/base"), |variant| variant.as_str()
                != "c/spell/base/mass")
            .is_empty()
        );
    }

    #[test]
    fn a_variants_row_reads_its_levels_from_the_spell_list_index() {
        let lists = SpellLists {
            spells: vec![
                cast_iron_schema::SpellListSpell(id("c/spell/base"), "Base".into()),
                cast_iron_schema::SpellListSpell(id("c/spell/variant"), "Variant".into()),
            ],
            classes: vec![
                SpellListClass {
                    name: "Druid".into(),
                    class: None,
                    levels: vec![cast_iron_schema::SpellListLevel {
                        level: 6,
                        spells: vec![1],
                    }],
                },
                SpellListClass {
                    name: "Bard".into(),
                    class: None,
                    levels: vec![cast_iron_schema::SpellListLevel {
                        level: 5,
                        spells: vec![1],
                    }],
                },
                SpellListClass {
                    name: "Cleric".into(),
                    class: None,
                    levels: vec![cast_iron_schema::SpellListLevel {
                        level: 5,
                        spells: vec![1],
                    }],
                },
                SpellListClass {
                    name: "Wizard".into(),
                    class: None,
                    levels: vec![cast_iron_schema::SpellListLevel {
                        level: 0,
                        spells: vec![1],
                    }],
                },
            ],
            details: vec![],
        };
        let rows = spell_level_rows(&lists, &id("c/spell/variant"));
        assert_eq!(
            rows,
            vec![
                ("Bard".into(), 5),
                ("Cleric".into(), 5),
                ("Druid".into(), 6),
                ("Wizard".into(), 0),
            ]
        );
        assert_eq!(
            levels_line(&rows).as_deref(),
            Some("Bard 5, Cleric 5, Druid 6, Wizard Cantrips")
        );
        assert!(spell_level_rows(&lists, &id("c/spell/missing")).is_empty());
        assert_eq!(levels_line(&[]), None);
    }

    #[test]
    fn a_variant_edge_never_joins_a_feat_chain() {
        let relations = labelled_edges(&[
            ("c/spell/variant", "variant", "c/spell/base", None),
            ("c/feat/follower", "prerequisite", "c/feat/base", None),
        ]);
        assert!(relations.leads_to(&id("c/spell/base")).is_empty());
        assert!(
            relations
                .prerequisite_chain(&id("c/spell/base"))
                .next
                .is_empty()
        );
        assert_eq!(
            relations.leads_to(&id("c/feat/base")),
            vec![&id("c/feat/follower")]
        );
    }

    #[test]
    fn a_spell_base_stacks_its_mythic_add_on_exactly_as_a_feat_does() {
        let relations = labelled_edges(&[
            (
                "c/spell/base/mythic",
                "addon",
                "c/spell/base",
                Some("mythic"),
            ),
            (
                "c/spell/base/stamina",
                "addon",
                "c/spell/base",
                Some("stamina"),
            ),
        ]);
        assert_eq!(
            addons_of_label(&relations, &id("c/spell/base"), "mythic", |_| true),
            [id("c/spell/base/mythic")]
        );
        // The panel reads the raw graph and tests only the add-on's id, as the feat page does.
        assert!(addons_of_label(&relations, &id("c/spell/base"), "mythic", |_| false).is_empty());
        // A spell page now loads the relation index that the mythic panel and Variants row need.
        assert!(needs(EntryKind::Spell).relations);
        assert!(needs(EntryKind::Spell).spell_lists);
    }

    #[test]
    fn a_stacked_add_on_body_loses_its_stat_block_and_demotes_its_headings() {
        let stat = Block::StatBlock {
            title: None,
            rows: vec![cast_iron_schema::StatRow {
                label: "Type".into(),
                value: vec![cast_iron_schema::Inline::text("Mythic")],
            }],
        };
        let heading = Block::Heading {
            level: 2,
            anchor: Some("benefits".into()),
            content: vec![cast_iron_schema::Inline::text("Benefits")],
        };
        let paragraph = Block::Paragraph {
            style: cast_iron_schema::ParagraphStyle::Normal,
            content: vec![cast_iron_schema::Inline::text("Text.")],
        };
        let out = attached_body(&[stat, heading, paragraph]);
        assert_eq!(out.len(), 2);
        assert!(matches!(&out[0], Block::Paragraph { content, .. }
            if matches!(content.as_slice(), [cast_iron_schema::Inline::Strong { .. }])));
        assert!(matches!(&out[1], Block::Paragraph { .. }));
        assert!(
            !out.iter()
                .any(|block| matches!(block, Block::StatBlock { .. } | Block::Heading { .. }))
        );
    }
}
