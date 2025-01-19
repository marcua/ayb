#!/usr/bin/env bash

git clone https://github.com/google/nsjail.git nsjail-checkout
cd nsjail-checkout
git checkout bc30a1a335bb4d5d2a24d1a7abeb420af9bb3388
make
mv nsjail ..
cd ..
rm -rf nsjail-checkout
