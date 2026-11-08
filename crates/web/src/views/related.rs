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
            relations: false,
            spell_lists: true,
            option_groups: false,
            replaces: false,
        },
        // The metadata line of an option names its group and, when the group has one class,
        // links that class.
        EntryKind::ClassOption => Needs {
            relations: false,
            spell_lists: false,
            option_groups: true,
            replaces: false,
        },
        EntryKind::Rule
        | EntryKind::Creature
        | EntryKind::Item
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
            let label = format!("{name} level {level}, {}", spells_noun(n));
            view! {
                <li>
                    <a class="lvl" href=href aria-label=label>
                        <span class="lvl-n">{level}</span>
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

/// The mythic add-ons that a base page stacks: the edge label picks them, and the add-on's own
/// book decides whether one appears. The base's own book never hides an add-on.
pub fn mythic_addons(
    relations: &Relations,
    base: &EntryId,
    eligible: impl Fn(&EntryId) -> bool,
) -> Vec<EntryId> {
    relations
        .addons_of(base, "mythic")
        .into_iter()
        .filter(|id| eligible(id))
        .cloned()
        .collect()
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

/// One mythic version inside a base page's `Mythic` section. The title and the source line
/// arrive with the page; only the rules text waits for the add-on's shard.
#[component]
fn MythicAddon(library: Library, id: EntryId) -> impl IntoView {
    let href = Route::entry(id.clone()).href();
    let title = title_of(&library, &id);
    let book = source_title(&library, &id);
    let load = library.clone();
    let body = local_resource(async move { load.entry(&id).await });
    view! {
        <div class="attached-addon">
            <h3><a href=href>{title}</a></h3>
            <p class="sources-line">{format!("Sources: {book}")}</p>
            {move || {
                body.with(|state| match state {
                    None => view! { <p class="status" role="status">"Loading the mythic rules…"</p> }.into_any(),
                    Some(Err(error)) => view! {
                        <p class="error" role="alert">{error.to_string()}</p>
                    }.into_any(),
                    Some(Ok(entry)) => view! {
                        <div class="rich">{blocks(&attached_body(&entry.body))}</div>
                    }.into_any(),
                })
            }}
        </div>
    }
}

fn mythic_panel(library: &Library, id: &EntryId, all: Option<&Relations>) -> Option<AnyView> {
    let relations = all?;
    let discovery = library.discovery();
    let addons = mythic_addons(relations, id, |addon| discovery.is_eligible(addon.as_str()));
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

/// The Stamina add-ons that a base page folds: the edge label picks them, and the add-on's own
/// book decides whether one appears. The base's own book never hides an add-on.
pub fn stamina_addons(
    relations: &Relations,
    base: &EntryId,
    eligible: impl Fn(&EntryId) -> bool,
) -> Vec<EntryId> {
    relations
        .addons_of(base, "stamina")
        .into_iter()
        .filter(|id| eligible(id))
        .cloned()
        .collect()
}

/// One collapsed Stamina disclosure on a base page. The summary names the add-on's book; the
/// rules text waits for the add-on's shard. No title, box or tint: the summary is the heading.
#[component]
fn StaminaAddon(library: Library, id: EntryId) -> impl IntoView {
    let book = source_title(&library, &id);
    let load = library.clone();
    let body = local_resource(async move { load.entry(&id).await });
    view! {
        <details class="rel-disclosure rel-stamina">
            <summary>{icons::chevron_right("disclosure-chevron")}{format!("Stamina · {book}")}</summary>
            {move || {
                body.with(|state| match state {
                    None => view! { <p class="status" role="status">"Loading the Stamina rules…"</p> }.into_any(),
                    Some(Err(error)) => view! {
                        <p class="error" role="alert">{error.to_string()}</p>
                    }.into_any(),
                    Some(Ok(entry)) => view! {
                        <div class="rich">{blocks(&attached_body(&entry.body))}</div>
                    }.into_any(),
                })
            }}
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
    stamina_addons(relations, id, |addon| discovery.is_eligible(addon.as_str()))
        .into_iter()
        .map(|addon| view! { <StaminaAddon library=library.clone() id=addon /> }.into_any())
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
            let text = format!("{} {}", l.class, l.level);
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
        EntryKind::Class => panels.extend(class_panels(
            &library,
            &entry.id,
            relations,
            lists,
            indexes.option_groups.as_deref(),
            indexes.replaces.as_deref(),
        )),
        EntryKind::Archetype => panels.extend(archetype_panel(&library, &entry)),
        EntryKind::PrestigeClass => panels.extend(prestige_panel(&library, &entry)),
        EntryKind::Feat => {
            panels.extend(mythic_panel(&library, &entry.id, all_relations));
            panels.extend(stamina_panel(&library, &entry.id, all_relations));
            panels.extend(feat_panels(&library, &entry, relations));
        }
        EntryKind::Trait => panels.extend(trait_panel(&entry)),
        EntryKind::Spell => panels.extend(spell_panel(&entry, lists)),
        EntryKind::Rule
        | EntryKind::Creature
        | EntryKind::Item
        | EntryKind::Skill
        | EntryKind::Affliction
        | EntryKind::Trap
        | EntryKind::ClassOption => {}
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
            source_ids: vec!["c".into()],
            section: vec![],
            parent: None,
            body: vec![],
            fields: TypedFields::Feat(FeatFields {
                prerequisite_feats: prereq_feats.iter().map(|s| id(s)).collect(),
                ..Default::default()
            }),
            tags: vec![],
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
            mythic_addons(&relations, &id("c/base"), |_| true),
            [id("c/base/mythic")]
        );
        // The add-on's book is what gates the section; the base's book never does.
        assert_eq!(
            mythic_addons(&relations, &id("c/base"), |addon| addon.as_str()
                != "c/base/mythic"),
            Vec::<EntryId>::new()
        );
        // A Stamina add-on is not a mythic version.
        assert!(mythic_addons(&relations, &id("c/other"), |_| true).is_empty());
    }

    #[test]
    fn a_base_page_folds_a_stamina_add_on_only_while_its_own_book_is_active() {
        let relations = labelled_edges(&[
            ("c/base/mythic", "addon", "c/base", Some("mythic")),
            ("c/base/stamina", "addon", "c/base", Some("stamina")),
        ]);
        assert_eq!(
            stamina_addons(&relations, &id("c/base"), |_| true),
            [id("c/base/stamina")]
        );
        // The add-on's own book gates the row; the base's book never does.
        assert_eq!(
            stamina_addons(&relations, &id("c/base"), |addon| addon.as_str()
                != "c/base/stamina"),
            Vec::<EntryId>::new()
        );
        // A mythic version is not a Stamina row.
        assert!(
            stamina_addons(&relations, &id("c/base"), |addon| addon.as_str()
                == "c/base/mythic")
            .is_empty()
        );
        // A base with no add-on gets no row.
        assert!(stamina_addons(&relations, &id("c/none"), |_| true).is_empty());
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
