#!/bin/bash

set -ex

git checkout main
git push
git checkout gh-pages
git merge main --no-edit
git rev-parse --short HEAD > version.txt
# git fails to recognize that version.txt was updated if we don't add it explicitly, weird…
git add version.txt

yarn compile-all

git commit -am "Publish updated version"
git push

git checkout main
