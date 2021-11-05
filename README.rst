wanikani-apprentice
===================

Setup
-----

.. code-block:: sh

   brew bundle install
   virtualenv -p $(brew --prefix python@3.10/bin/python3.10) ~/.virtualenvs/wanikani_apprentice
   source ~/.virtualenvs/wanikani_apprentice/bin/activate
   pip install 'pip >= 21.3'
   pip install -r requirements.txt
   overmind start

Deploying
---------

Deploys are done automatically on GitHub Actions, but can also be run manually:

.. code-block:: sh

   flyctl deploy

Keeping up to date with template changes
----------------------------------------

.. code-block:: sh

   # Add the 'template' remote if necessary
   git remote get-url template || git remote add template git@github.com:rouge8/python-template.git

   # Fetch and merge the latest changes
   git fetch template
   git merge template/main

   # Resolve any merge conflicts, run the tests, commit your changes, etc.
