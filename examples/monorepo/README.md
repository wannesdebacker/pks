# Example monorepo

A tiny npm-workspaces monorepo to try `pks` on. Every script just prints where it ran.

```sh
cd examples/monorepo/apps/web
pks            # local scripts first, then workspace, then the other packages
pks -l         # same, as a list
pks lint       # runs the workspace "lint" script from the repo root
```
