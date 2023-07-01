[//]: # SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
[//]: # SPDX-License-Identifier: EUPL-1.2

# serves3

A **very** simple proxy to browse files from private S3 buckets.

Helpful to be put behind another authenticating web server, such as Apache or NGINX.

Also helpful to do a different TLS termination.

## Configuration

Copy `Settings.toml.example` to `Settings.toml` and adjust your settings.

You can also add a `Rocket.toml` file to customize the server options. See the [Rocket documentation](https://rocket.rs/v0.5-rc/guide/configuration/#rockettoml).

Then just configure Apache or NGINX to proxy to the given port. For example:

```apache
<VirtualHost *:443>
    ServerName example.com
    ServerAdmin support@example.com
    DocumentRoot /var/www

    ProxyPreserveHost On
    ProxyPass /s3/ http://127.0.0.1:8000/
    ProxyPassReverse /s3/ http://127.0.0.1:8000/

    # ... other options ...
</VirtualHost>

```

## Build and install

```bash
cargo install --root /usr/local # for instance
cd run-folder # folder with Settings.toml
serves3
```
