#!/bin/bash

set -ex

git checkout main
git push
git checkout gh-pages
git merge main --no-edit
git rev-parse --short HEAD > version.txt

yarn compile-all

git commit -am "Publish updated version"
git push
