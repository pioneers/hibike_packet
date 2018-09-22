#!/bin/bash
python3.7 -m pipenv run python setup.py bdist_wheel
cp -r dist artefacts
