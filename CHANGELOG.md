# Version 0.2.0 (2024-08-14)

## new feature

1. select hentai

   ```bash
   nhentai-rs download --name <name> [--interaction true/false]
   ```

   if `--interaction` is true, you can select hentai after search hentai,
   if it is option, program will enable select list by interaction parameter in config.yaml


# Version 0.1.0 (2024-08-12)

1. generate profile

   ```bash
   nhentai-rs generate
   ```

2. download 

   ```bash
   nhentai-rs download --name <name>
   ```

3. convert to pdf

   ```bash
   nhentai-rs convert --path <path> --name <name> [--dir <dir>]
   ```

4. compress

   ```bash
   nhentai-rs compress --path <path> --name <name> [--secret <secret> --dir <dir>]
   ```