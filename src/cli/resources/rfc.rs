use clap::Subcommand;

use crate::model::RfcPhase;
use crate::{
    CommonAddArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs, CommonListArgs,
    CommonRemoveArgs, CommonRenderArgs, CommonSetArgs, CommonShowArgs, CommonSupersedeArgs,
    FinalizeStatus,
};

/// RFC commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum RfcCommand {
    /// List RFCs
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be an RFC status, phase, or ID/title substring.

EXAMPLES:
    govctl rfc list
    govctl rfc list draft
    govctl rfc list impl -n 5
    govctl rfc list RFC-0002 -o json
"
    )]
    List(CommonListArgs),
    /// Get RFC metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, version, status, phase, owners, refs, changelog

EXAMPLES:
    govctl rfc get RFC-0001
    govctl rfc get RFC-0001 title
    govctl rfc get RFC-0001 refs
")]
    Get(CommonGetArgs),
    /// Show rendered RFC content
    #[command(after_help = "\
EXAMPLES:
    govctl rfc show RFC-0001
    govctl rfc show RFC-0001 -o plain

NOTES:
    - `show` prints human-readable rendered content.
    - Use `get` for field/path-level inspection.
")]
    Show(CommonShowArgs),
    /// Create a new RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc new \"Add incremental index rebuilding\"
    govctl rfc new \"Add incremental index rebuilding\" --id RFC-0010

NOTES:
    - Use `--id` only when you need to pin a specific RFC ID.
    - New RFCs start as draft and can later be finalized.
")]
    New {
        /// RFC title
        title: String,
        /// RFC ID (e.g., RFC-0010). Auto-generated if omitted.
        #[arg(long)]
        id: Option<String>,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl rfc edit RFC-0001 changelog.summary --set \"Clarify retry behavior\"
    govctl rfc edit RFC-0001 changelog.fixed --add \"Correct timeout wording\"
    govctl rfc edit RFC-0001 changelog.fixed[0] --remove
    govctl rfc edit RFC-0001 refs --add RFC-0002

NOTES:
    - Changelog edits apply only to the entry matching the RFC's current version.
    - RFC and changelog version/date fields are lifecycle-owned.
")]
    Edit(CommonEditArgs),
    /// Set RFC field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - title: RFC title

  Array fields (use 'add' / 'remove'):
    - owners, refs, sections

EXAMPLES:
    govctl rfc set RFC-0001 title \"New Title\"

Use dedicated lifecycle verbs instead of `set` for:
    - version → `govctl rfc bump`
    - status → `govctl rfc finalize` / `govctl rfc deprecate` / `govctl rfc supersede`
    - phase → `govctl rfc advance`
")]
    Set(CommonSetArgs),
    /// Add value to RFC array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to other RFCs (e.g., \"RFC-0002\")
    - owners: RFC owners (e.g., \"@alice\")

EXAMPLES:
    govctl rfc add RFC-0001 refs RFC-0002
    govctl rfc add RFC-0001 owners @alice
")]
    Add(CommonAddArgs),
    /// Remove value from RFC array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, owners

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl rfc remove RFC-0001 refs RFC-0002     # Remove first match
    govctl rfc remove RFC-0001 refs --at 1       # Remove by index
")]
    Remove(CommonRemoveArgs),
    /// Bump RFC version
    #[command(after_help = "\
EXAMPLES:
    govctl rfc bump RFC-0001 --patch -m \"Clarify examples\"
    govctl rfc bump RFC-0001 --minor -m \"Add a normative clause\" -c \"change: Define the new behavior\"
    govctl rfc bump RFC-0001 -c \"fix: Correct current-version wording\"

NOTES:
    - Version-changing bumps require a normative RFC in impl, test, or stable with a sealed signature.
    - While an RFC is in spec, continue authoring the current version candidate instead of bumping again.
    - Choose one of `--patch`, `--minor`, or `--major` when releasing a content amendment.
    - `--change` without a bump level updates the current changelog entry without changing version.
    - Use `-m/--summary` for a release summary and `-c/--change` for detailed entries.
")]
    Bump {
        /// RFC ID
        id: String,
        /// Patch version bump
        #[arg(long, group = "bump_level")]
        patch: bool,
        /// Minor version bump
        #[arg(long, group = "bump_level")]
        minor: bool,
        /// Major version bump
        #[arg(long, group = "bump_level")]
        major: bool,
        /// Changelog summary
        #[arg(short = 'm', long)]
        summary: Option<String>,
        /// Add change description(s)
        #[arg(short = 'c', long = "change")]
        changes: Vec<String>,
    },
    /// Finalize RFC status (draft → normative)
    #[command(after_help = "\
EXAMPLES:
    govctl rfc finalize RFC-0001 normative

NOTES:
    - Use `deprecate` for normative → deprecated.
    - Use `advance` to move phase after finalization.
")]
    Finalize {
        /// RFC ID
        id: String,
        /// Target status (`normative`)
        #[arg(value_enum)]
        status: FinalizeStatus,
    },
    /// Advance RFC phase
    #[command(after_help = "\
EXAMPLES:
    govctl rfc advance RFC-0001 impl
    govctl rfc advance RFC-0001 test

NOTES:
    - Typical progression is `spec -> impl -> test -> stable`.
    - Use this after the RFC has been finalized.
")]
    Advance {
        /// RFC ID
        id: String,
        /// Target phase
        #[arg(value_enum)]
        phase: RfcPhase,
    },
    /// Deprecate RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc deprecate RFC-0001
    govctl rfc deprecate RFC-0001 --force
")]
    Deprecate(CommonDeprecateArgs),
    /// Supersede RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc supersede RFC-0001 --by RFC-0002
    govctl rfc supersede RFC-0001 --by RFC-0002 --force
")]
    Supersede(CommonSupersedeArgs),
    /// Render a single RFC to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl rfc render RFC-0001
    govctl rfc render RFC-0001 --dry-run
")]
    Render(CommonRenderArgs),
}
