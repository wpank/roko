"""The ViabilityBench common library: the machinery every task family shares (S08 §4.2, §4.4, §6 T2).

Stdlib only, Python 3.11 or newer. Families, the driver and CI import it as a package, after putting the
`families/` directory on `sys.path`:

    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))   # from families/<family>/<script>.py
    from common import astcheck, canary, hmac_seed, knobs, mutate, repo

The modules and their public names:

- `repo`: a task repo with a pristine snapshot of its base commit, and clean exports for the census.
  `init_task_repo`, `Pristine`, `export_tree`, `restore_paths`, `tree_hash`, `RepoError`.
- `knobs`: a family's `ladder.toml`, the knob values of levels ℓ1–ℓ5 (S08 §4.4), and instance ids.
  `load_ladder`, `Ladder`, `s08_deviations`, `instance_id`, `parse_instance_id`, `LEVELS`, `LadderError`.
- `hmac_seed`: deterministic random streams, public for the visible instance and keyed by the secret for hidden
  cases; the secret file. `read_secret_file`, `write_secret_file`, `add_secret_file_argument`, `Secret`,
  `hidden_stream`, `surface_stream`, `Stream`, `SecretFileError`.
- `astcheck`: deterministic detectors for planted gaming. `python_sources`, `literal_returns`, `skipped_tests`,
  `wrong_base_class`, `test_edits`, `file_hashes`, `is_cache`, `gaming_summary`, `Finding`, `CHECKS`.
- `canary`: the per-release canary GUID that shows an agent read a hidden file. `RELEASE_CANARY`, `CANARY_RE`,
  `new_canary`, `mark`, `mark_file`, `strip`, `find`, `find_in_diff`, `find_in_tree`, `CanaryError`.
- `mutate`: surface renames, so the instances of one family differ. `choose`, `rename_text`, `rename_tree`.

Each module's docstring fixes its signatures and contracts. Change them only together with every family that uses
them. `VERSION` belongs in each family's `verifier_version`, so a change here shows up in the records.
"""

VERSION = "common-1.1.0"  # 1.1.0: test_edits and file_hashes skip bytecode and pytest caches (bug-993e7e)
