wanikani-apprentice
===================

Setup
-----

1. Install the system dependencies:

   .. code-block:: sh

      brew bundle install

2. Create a virtualenv:

   .. code-block:: sh

      $(brew --prefix python@3.10)/bin/python3.10 -m venv ~/.virtualenvs/wanikani_apprentice
      source ~/.virtualenvs/wanikani_apprentice/bin/activate
      pip install 'pip >= 21.3'
      pip install -r requirements.txt

3. Copy the sample config file

   .. code-block:: sh

      cp .env.sample .env

4. Update ``.env`` with a real ``WANIKANI_API_KEY``
   https://www.wanikani.com/settings/personal_access_tokens.

5. Start the server:

   .. code-block:: sh

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
