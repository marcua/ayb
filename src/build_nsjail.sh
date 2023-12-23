#!/usr/bin/env bash

git clone https://github.com/google/nsjail.git nsjail-checkout
cd nsjail-checkout
make
mv nsjail ..
cd ..
rm -rf nsjail-checkout

