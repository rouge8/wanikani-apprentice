wanikani-apprentice
===================

Setup
-----

1. Copy the sample config file

   ```sh
   cp .env.sample .env
   ```

2. Update ``.env`` with a real ``WANIKANI_API_KEY``
   https://www.wanikani.com/settings/personal_access_tokens.

3. Start the server:

   ```sh
   cargo run
   ```

Deploying
---------

Deploys are done automatically on GitHub Actions, but can also be run manually:

```sh
flyctl deploy
```
