"""Keep pytest out of the feature templates: their tests import packages that exist only in a rendered task repo."""

collect_ignore_glob = ["features/*"]
