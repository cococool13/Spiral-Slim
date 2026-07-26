#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import sys

from browser_collection.adapters.brave import BraveAdapter
from browser_collection.custom import CustomProfileError
from browser_collection.engine import CollectionEngine, EngineError
from browser_collection.evidence import EvidenceError
from browser_collection.registry import Registry
from browser_collection.render import preview_to_dict, render_preview_text
from browser_collection.resolver import ResolutionError
from browser_collection.runner import SubprocessRunner
from browser_collection.schema import ConfigError


ROOT = Path(__file__).resolve().parent


def current_platform():
    if sys.platform == "darwin":
        return "macos"
    if sys.platform == "win32":
        return "windows"
    return "unsupported"


def build_engine():
    return CollectionEngine(
        Registry.load(ROOT),
        {"brave": BraveAdapter(SubprocessRunner())},
        current_platform(),
    )


def build_parser():
    parser = argparse.ArgumentParser(
        description=(
            "Inspect Spiral browser profiles without changing the system."
        )
    )
    actions = parser.add_mutually_exclusive_group()
    actions.add_argument("--catalog", action="store_true")
    actions.add_argument("--detect", action="store_true")
    actions.add_argument("--preview", metavar="PROFILE_ID")
    actions.add_argument(
        "--preview-custom",
        dest="preview_custom",
        action="store_true",
        help="Preview a selection composed from --modules.",
    )
    parser.add_argument(
        "--modules",
        help="Comma-separated module ids for --preview-custom.",
    )
    parser.add_argument(
        "--exclude",
        help=(
            "Comma-separated control ids to leave at Brave's default. "
            "Required controls cannot be excluded."
        ),
    )
    parser.add_argument(
        "--browser",
        help="Browser id, comma-separated ids, or all (default: all).",
    )
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
    )
    return parser


def parse_args(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if not (
        args.catalog
        or args.detect
        or args.preview is not None
        or args.preview_custom
    ):
        parser.error(
            "one of the arguments --catalog --detect --preview "
            "--preview-custom is required"
        )
    if args.modules is not None and not args.preview_custom:
        parser.error("--modules is only valid with --preview-custom")
    if args.exclude is not None and not args.preview_custom:
        parser.error("--exclude is only valid with --preview-custom")
    if args.preview_custom and not args.modules:
        parser.error("--preview-custom requires --modules")
    return args


def parse_id_list(selection, label):
    """Split a comma-separated id list, rejecting blanks and duplicates."""
    if selection is None:
        return []
    items = tuple(item.strip() for item in selection.split(","))
    if not items or any(not item for item in items):
        raise ConfigError(f"{label} must not contain an empty id")
    if len(items) != len(set(items)):
        raise ConfigError(f"{label} must not repeat an id")
    return list(items)


def parse_browser_ids(selection):
    if selection is None or selection == "all":
        return None
    items = tuple(item.strip() for item in selection.split(","))
    if not items or any(not item for item in items):
        raise ConfigError(
            "browser selection must contain at least one browser id"
        )
    if "all" in items:
        raise ConfigError("'all' cannot be combined with browser ids")
    return list(dict.fromkeys(items))


def installation_to_dict(installation):
    return {
        "browser_id": installation.browser_id,
        "name": installation.name,
        "platform": installation.platform,
        "path": installation.path,
        "version": installation.version,
    }


def render_catalog_text(payload):
    return "\n".join(
        [payload["tool"]["name"]]
        + [
            f"- {item['id']}: {item['name']}"
            for item in payload["profiles"]
        ]
    )


def render_detection_text(payload):
    return "\n".join(
        f"{browser_id}: {len(installations)} detected"
        for browser_id, installations in payload.items()
    )


def emit(payload, output_format, text_renderer):
    if output_format == "json":
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print(text_renderer(payload))


def main(argv=None):
    args = parse_args(argv)
    try:
        if args.catalog and args.browser is not None:
            raise ConfigError(
                "--browser is only valid with --detect or --preview"
            )
        browser_ids = (
            None if args.catalog else parse_browser_ids(args.browser)
        )
        engine = build_engine()
        if args.catalog:
            payload = engine.catalog()
            emit(payload, args.format, render_catalog_text)
            return 0
        if args.detect:
            payload = {
                browser_id: [
                    installation_to_dict(item)
                    for item in installations
                ]
                for browser_id, installations
                in engine.detect(browser_ids).items()
            }
            emit(payload, args.format, render_detection_text)
            return 0

        if args.preview_custom:
            result = engine.preview_custom(
                parse_id_list(args.modules, "--modules"),
                parse_id_list(args.exclude, "--exclude"),
                browser_ids,
            )
        else:
            result = engine.preview(args.preview, browser_ids)
        if args.format == "json":
            print(
                json.dumps(
                    preview_to_dict(result),
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print(render_preview_text(result))
        return 3 if result.blocked else 0
    except (
        ConfigError,
        CustomProfileError,
        ResolutionError,
        EngineError,
        EvidenceError,
        KeyError,
    ) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
