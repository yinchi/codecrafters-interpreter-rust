This is my Rust code for the CodeCrafters
["Build your own Interpreter" Challenge](https://app.codecrafters.io/courses/interpreter/overview).

This challenge follows the book
[Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom.

## Pushing to GitHub

CodeCrafters generates new Git commits for each challenge task submission. To avoid pushing all commits to GitHub, we use squash commits:

```sh
# Switch to the gh-publish branch
git switch gh-publish

# Squash all commits from master into one commit on gh-publish
git merge master --squash --allow-unrelated-histories -X theirs
git commit -m "<Message for squashed commit>"

# Push the squashed commit to GitHub
git push github gh-publish

# Switch back to master branch
git switch master
```
