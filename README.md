This is my Rust code for the CodeCrafters
["Build your own Interpreter" Challenge](https://app.codecrafters.io/courses/interpreter/overview).

This challenge follows the book
[Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom.

## Pushing to GitHub

CodeCrafters generates new Git commits for each challenge task submission. To avoid pushing all commits to GitHub, we use squash commits:

```sh
git switch gh-publish
git merge origin/master --squash --allow-unrelated-histories
git push -m "<Message for squashed commit>"
git switch master
```
