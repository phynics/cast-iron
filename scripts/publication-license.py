"""Pure license-basis checks used by the publication screen."""

COMMUNITY_USE = "legal#community-use"


def cup_publications(license_rows):
    """Return the publications whose reviewed legal basis names the Community Use Policy.

    `license_rows` are the `publication_licenses` rows of the corpus `legal.toml`.
    """
    return {
        row["publication_id"]
        for row in license_rows
        if COMMUNITY_USE in row["license_refs"]
    }


def has_license_basis(entry, cup_sources):
    """Return whether an entry carries a reviewed license basis.

    An entry with the OGL reference passes. A Community Use only entry (decision
    2026-11-18, Paizo Blog source bases) passes only when it carries the Community
    Use path, cites at least one source, and every source it cites is in
    `cup_sources`. Whether each source is reviewed at all is a separate check.
    """
    if "ogl-1.0a" in entry["license_refs"]:
        return True
    source_ids = entry["source_ids"]
    return (
        COMMUNITY_USE in entry["license_refs"]
        and bool(source_ids)
        and all(source in cup_sources for source in source_ids)
    )
