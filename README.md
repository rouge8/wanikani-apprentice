wanikani-apprentice
===================

Setup
-----

1. Install the system dependencies:

   ```sh
   brew bundle install
   ```

2. Copy the sample config file

   ```sh
   cp .env.sample .env
   ```

3. Update ``.env`` with a real ``WANIKANI_API_KEY``
   https://www.wanikani.com/settings/personal_access_tokens.

4. Start the server:

   ```sh
   overmind start
   ```

Deploying
---------

Deploys are done automatically on GitHub Actions, but can also be run manually:

```sh
flyctl deploy
```
