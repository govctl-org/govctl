# Parallel Workspaces

When several agents work in one clone through separate git worktrees or jj
workspaces, govctl coordinates them so they do not collide on artifact IDs,
change the same RFC's version at once, or cut releases from the wrong place.
The rules are defined in RFC-0010.

## When Coordination Applies

Coordination state lives in the clone's shared version-control storage (the
git common directory or the jj repository store). It is never committed and
never changes governed artifacts.

- **Single checkout:** behavior is unchanged.
- **No version control:** multiple workspaces cannot exist, so commands behave
  as they always have, silently.
- **Version control present but the shared state is unreadable:** commands
  continue in single-checkout mode and warn (`W0115`).

## Artifact IDs

IDs are reserved across workspaces when an artifact is created, so two
branches never produce the same RFC, ADR, or work item number, whichever
merges first. Allocation also consults shared history, so the ID of a deleted
artifact is not reused.

## RFC Claims

Operations that change an RFC's version semantics take an exclusive claim on
that RFC: `bump`, `finalize`, `advance`, `deprecate`, and `supersede`
(supersession claims both RFCs). If another workspace holds a live claim, the
command fails with `E0824` and names that workspace.

Content edits, including clause authoring, are never blocked. They warn with
`W0116` when another workspace holds a claim, so overlapping work is visible.

```bash
govctl claim list             # Claims held across this clone's workspaces
govctl claim release RFC-0001 # Release a claim this workspace holds
govctl claim steal RFC-0001   # Take over another workspace's claim
```

Every `steal` is recorded as an audit event naming both workspaces. A claim
expires, and stops blocking, when its workspace no longer exists or has been
inactive for `claim_ttl_days` (default 7):

```toml
[workspace]
claim_ttl_days = 7
```

## Seeing Work in Other Workspaces

Activating a work item records that this workspace is working on it.
`govctl status` lists work items active in other workspaces under **Active in
Other Workspaces**, with each owning workspace's path. This is visibility only:
it never blocks activating or editing the same item elsewhere.

## The Primary Workspace

Some commands rewrite the project's shared history and must run from one
place, the primary workspace:

- **git:** the main working tree.
- **jj:** the workspace named by `workspace.primary`, or `default` when it is
  not configured.

```toml
[workspace]
primary = "default"
```

## Trunk-Scoped Commands

`govctl release`, `govctl release undo`, and `govctl migrate` are trunk-scoped.
Everything else runs in any workspace.

Run from a secondary workspace, a trunk-scoped command refuses with `E0823` and
names the primary workspace, by path or, when jj has not recorded its root, by
workspace name. Nothing is changed.

When govctl cannot determine the primary workspace, the command runs as if the
current workspace were primary and warns with `W0114`. The warning states the
cause and a matching hint:

| Cause                                      | Hint                                           |
| ------------------------------------------ | ---------------------------------------------- |
| `git` or `jj` is missing or failing        | Make sure the tool works in this directory     |
| Bare repository or separate git directory  | Run from a clone with a main working tree      |
| jj cannot single out the current workspace | Give it its own working-copy commit (`jj new`) |
| No jj workspace has the primary name       | Set `workspace.primary` to a listed workspace  |

jj workspaces created before jj 0.38 do not record their root path. govctl
still recognizes them by name and working-copy commit, so enforcement works
unless two such workspaces share the same working-copy commit.
