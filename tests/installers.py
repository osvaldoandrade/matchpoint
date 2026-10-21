"""Installer contracts with real Git/filesystem and fake external build tools."""
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import tarfile
import unittest

INSTALLER = Path(os.environ.get("MATCHPOINT_TEST_INSTALLER", Path(__file__).resolve().parents[1] / "install.sh"))


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="matchpoint-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "origin"
        self.repo.mkdir()
        for args in [("init", "-b", "main"), ("config", "user.name", "Installer Test"),
                     ("config", "user.email", "test@example.invalid")]:
            self.git(*args)
        (self.repo / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.94.0"\n')
        (self.repo / ".gitignore").write_text("/target/\n")
        self.git("add", ".")
        self.git("commit", "-m", "test: create source fixture")
        self.prefix = self.root / "user's apps $literal"

    def git(self, *args):
        return subprocess.run(["git", "-C", str(self.repo), *args], check=True,
                              text=True, capture_output=True).stdout.strip()

    def run_shell(self, body, success=True):
        result = subprocess.run(["bash", "-c", 'source "$1"\n' + body,
                                 "test", str(INSTALLER)], text=True, capture_output=True,
                                env={**os.environ, "INSTALL_TEST_ROOT": str(self.root),
                                     "INSTALL_TEST_PREFIX": str(self.prefix),
                                     "INSTALL_TEST_ORIGIN": self.repo.as_uri()})
        if success:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def install(self, before="", success=True):
        return self.run_shell('''
INSTALL_REPOSITORY=$INSTALL_TEST_ORIGIN
system_dependencies() { :; }
rust_toolchain() { INSTALL_RUST=1.94.0; INSTALL_CARGO=(fake_cargo); }
id() { printf '1000\n'; }
fake_cargo() {
    [[ ${BUILD_FAIL:-0} == 0 ]] || return 19
    mkdir -p target/release
    printf '#!/usr/bin/env bash\nprintf "argument=%%s\\n" "$@"\n' > target/release/pgm-data
    chmod +x target/release/pgm-data
}
''' + before + '\nmain --prefix "$INSTALL_TEST_PREFIX" --no-path\n', success)

    def test_install_repeat_update_and_argument_forwarding(self):
        self.install()
        launcher = self.prefix / "bin/matchpoint"
        first = launcher.resolve()
        result = subprocess.run([str(launcher), "a b", "--version"], text=True,
                                check=True, capture_output=True)
        self.assertEqual(result.stdout, "argument=a b\nargument=--version\n")
        self.install()
        (self.repo / "next").write_text("version two")
        self.git("add", "next")
        self.git("commit", "-m", "test: update source fixture")
        self.install()
        self.assertNotEqual(launcher.resolve(), first)
        self.assertTrue(first.is_file())
        receipt = (self.prefix / "share/matchpoint/installation.txt").read_text()
        self.assertIn("commit=" + self.git("rev-parse", "HEAD"), receipt)
        self.assertFalse((self.prefix / "lib/matchpoint/install.lock").exists())

    def test_failed_build_preserves_previous_installation(self):
        self.install()
        launcher = self.prefix / "bin/matchpoint"
        previous = launcher.resolve().read_bytes()
        self.assertIn("stage=build", self.install("BUILD_FAIL=1", success=False).stdout)
        self.assertEqual(launcher.resolve().read_bytes(), previous)
        self.assertFalse((self.prefix / "lib/matchpoint/install.lock").exists())

    def test_foreign_executable_is_preserved(self):
        directory = self.prefix / "bin"
        directory.mkdir(parents=True)
        (directory / "pgm-data").write_text("owned by another installer")
        result = self.install(success=False)
        self.assertIn("unrelated executable", result.stderr)
        self.assertEqual((directory / "pgm-data").read_text(), "owned by another installer")

    def test_foreign_version_namespace_is_preserved(self):
        version = self.prefix / "lib/matchpoint/versions" / self.git("rev-parse", "HEAD")
        version.mkdir(parents=True)
        foreign = version / "pgm-data"
        foreign.write_text("keep foreign version")
        self.assertIn("unmanaged installation directory", self.install(success=False).stderr)
        self.assertEqual(foreign.read_text(), "keep foreign version")

    def test_failed_temporary_directory_creation_releases_lock(self):
        self.install('TMPDIR="$INSTALL_TEST_ROOT/missing"', success=False)
        self.assertFalse((self.prefix / "lib/matchpoint/install.lock").exists())

    def test_concurrent_install_does_not_remove_other_installers_lock(self):
        self.install()
        lock = self.prefix / "lib/matchpoint/install.lock"
        lock.mkdir()
        self.assertIn("Another installation", self.install(success=False).stderr)
        self.assertTrue(lock.is_dir())

    def test_stored_origin_is_used_with_git_url_rewrites(self):
        self.install()
        source = self.prefix / "share/matchpoint/source"
        subprocess.run(["git", "-C", str(source), "config", "url.file:///missing/.insteadOf",
                        self.repo.as_uri()], check=True)
        result = self.install(success=False)
        self.assertNotIn("origin does not match", result.stderr)

    def test_dirty_or_unmanaged_clone_is_preserved(self):
        self.install()
        source = self.prefix / "share/matchpoint/source"
        (source / "local-edit").write_text("keep me")
        self.assertIn("local edits", self.install(success=False).stderr)
        self.assertEqual((source / "local-edit").read_text(), "keep me")
        (source / "local-edit").unlink()
        subprocess.run(["git", "-C", str(source), "config", "--unset", "matchpoint.managed"], check=True)
        self.assertIn("unmanaged", self.install(success=False).stderr)

    def test_profiles_are_quoted_idempotent_and_symlinks_preserved(self):
        self.prefix.mkdir()
        profile = self.root / "profile"
        profile.write_text("# existing settings\n")
        linked = self.root / "linked-profile"
        linked.symlink_to(profile)
        self.run_shell('''
INSTALL_PREFIX=$INSTALL_TEST_PREFIX
mkdir -p "$INSTALL_PREFIX/share/matchpoint"
configure_path "$INSTALL_TEST_ROOT/profile" "$INSTALL_TEST_ROOT/linked-profile"
configure_path "$INSTALL_TEST_ROOT/profile"
source "$INSTALL_TEST_ROOT/profile"
[[ "$PATH" == "$INSTALL_PREFIX/bin:"* ]]
''')
        self.assertEqual(profile.read_text().count("# MatchPoint installer"), 1)
        self.assertTrue(linked.is_symlink())

    def test_invalid_arguments_never_create_an_installation(self):
        for args in ["--ref ../bad", "--ref --bad", "--prefix /", "--prefix relative", "--unknown", "--ref"]:
            with self.subTest(args=args):
                self.run_shell("arguments " + args, success=False)
        self.assertFalse(self.prefix.exists())

    def test_stdin_entry_point_supports_curl_pipe(self):
        result = subprocess.run(["bash", "-s", "--", "--help"], input=INSTALLER.read_text(),
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Usage:", result.stdout)

    def test_missing_git_ref_never_publishes_a_launcher(self):
        self.install("INSTALL_REF=missing-branch", success=False)
        self.assertFalse((self.prefix / "bin/matchpoint").exists())

    def test_checksum_mismatch_and_missing_record_are_rejected(self):
        data = self.root / "asset.tar.gz"
        data.write_bytes(b"downloaded archive")
        checksum = self.root / "checksums"
        checksum.write_text(hashlib.sha256(data.read_bytes()).hexdigest() + "  asset.tar.gz\n")
        command = 'verify_checksum "$INSTALL_TEST_ROOT/asset.tar.gz" "$INSTALL_TEST_ROOT/checksums"'
        self.run_shell(command)
        data.write_bytes(b"tampered")
        self.assertIn("mismatch", self.run_shell(command, success=False).stderr)
        checksum.write_text("0" * 64 + "  another-asset\n")
        self.assertIn("not found", self.run_shell(command, success=False).stderr)

    def test_private_access_failure_is_explicit(self):
        result = self.run_shell('''
git() { return 1; }
github_cli() { :; }
gh() { return 0; }
source_access
''', success=False)
        self.assertIn("cannot read osvaldoandrade/matchpoint", result.stderr)

    def test_existing_toolchain_is_reused_without_install(self):
        self.run_shell('''
INSTALL_SOURCE=$INSTALL_TEST_ROOT/origin
rustup() { [[ $1 == run ]]; }
rust_toolchain
[[ ${INSTALL_CARGO[*]} == 'cargo +1.94.0' ]]
''')

    def test_missing_toolchain_is_installed_at_pinned_version(self):
        self.run_shell('''
INSTALL_SOURCE=$INSTALL_TEST_ROOT/origin
rustup() {
    [[ $1 != run ]] || return 1
    [[ "$*" == 'toolchain install 1.94.0 --profile minimal' ]]
}
rust_toolchain
''')

    def test_missing_build_tools_can_be_reported_or_bootstrapped(self):
        body = '''
INSTALL_OS=Linux
need() {
    case "$1" in cc|c++|cmake) [[ ${TOOLS_READY:-0} == 1 ]] ;;
        apt-get) return 0 ;; *) command -v "$1" >/dev/null ;; esac
}
privileged() { TOOLS_READY=1; }
'''
        self.run_shell(body + 'INSTALL_SYSTEM_DEPS=0; system_dependencies', success=False)
        self.run_shell(body + 'system_dependencies; [[ $TOOLS_READY == 1 ]]')

    def test_github_cli_bootstrap_verifies_and_installs_archive(self):
        executable = self.root / "gh"
        executable.write_text("#!/bin/sh\nprintf 'GitHub CLI fixture\\n'\n")
        executable.chmod(0o755)
        asset = self.root / "gh_2.102.1_linux_amd64.tar.gz"
        with tarfile.open(asset, "w:gz") as archive:
            archive.add(executable, arcname="gh_2.102.1_linux_amd64/bin/gh")
        (self.root / "gh_2.102.1_checksums.txt").write_text(
            hashlib.sha256(asset.read_bytes()).hexdigest() + "  " + asset.name + "\n")
        result = self.run_shell('''
INSTALL_PREFIX=$INSTALL_TEST_PREFIX
INSTALL_TEMP=$INSTALL_TEST_ROOT/downloads
mkdir "$INSTALL_TEMP"
INSTALL_OS=Linux
uname() { printf 'x86_64\n'; }
need() { [[ $1 != gh ]] && command -v "$1" >/dev/null; }
download() { cp "$INSTALL_TEST_ROOT/${1##*/}" "$2"; }
github_cli
"$INSTALL_PREFIX/lib/matchpoint/tools/gh" --version
''')
        self.assertIn("GitHub CLI fixture", result.stdout)

    def test_rustup_bootstrap_when_rust_is_absent(self):
        bootstrap = self.root / "rustup-bootstrap.sh"
        bootstrap.write_text('''#!/bin/sh
set -eu
mkdir -p "$CARGO_HOME/bin"
cat > "$CARGO_HOME/bin/rustup" <<'RUSTUP'
#!/bin/sh
if [ "$1" = run ]; then exit 1; fi
[ "$*" = 'toolchain install 1.94.0 --profile minimal' ]
RUSTUP
chmod +x "$CARGO_HOME/bin/rustup"
''')
        self.run_shell('''
INSTALL_SOURCE=$INSTALL_TEST_ROOT/origin
INSTALL_TEMP=$INSTALL_TEST_ROOT
export CARGO_HOME=$INSTALL_TEST_ROOT/cargo
need() {
    case "$1" in rustup) [[ -x "$CARGO_HOME/bin/rustup" ]] ;;
        cargo|rustc) return 1 ;; *) command -v "$1" >/dev/null ;; esac
}
download() { cp "$INSTALL_TEST_ROOT/rustup-bootstrap.sh" "$2"; }
rust_toolchain
[[ -x "$CARGO_HOME/bin/rustup" ]]
''')


if __name__ == "__main__":
    unittest.main()
