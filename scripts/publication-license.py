"""Pure license-basis checks used by the publication screen."""

COMMUNITY_USE = "legal#community-use"


def has_reviewed_license_basis(entry, known_sources, cup_publications):
    """Return whether an entry's sources and license references are reviewed."""
    source_ids = entry["source_ids"]
    if not source_ids or not set(source_ids) <= known_sources:
        return False
    if "ogl-1.0a" in entry["license_refs"]:
        return True
    return COMMUNITY_USE in entry["license_refs"] and all(
        source in cup_publications for source in source_ids
    )
