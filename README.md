This is my Rust code for the CodeCrafters
["Build your own Interpreter" Challenge](https://app.codecrafters.io/courses/interpreter/overview).

This challenge follows the book
[Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom.

## Pushing to GitHub

CodeCrafters generates new Git commits for each challenge task submission. To avoid pushing all commits to GitHub, we use squash commits.

First, we need to set up the GitHub repository as a remote and create a new branch for publishing.
Once the local and remote branches are set up, we can use the following commands to set the upstream branches:

```sh
git branch --set-upstream-to=origin/master master
git branch --set-upstream-to=github/gh-publish gh-publish
```

Then, after finishing a task, we can push the changes to GitHub with the following commands:

```sh
git switch gh-publish
git merge origin/master --squash --allow-unrelated-histories
git commit -m "<Message for squashed commit>"
git push github gh-publish
git switch master
```
