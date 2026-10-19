// Ported from: glean/db/Glean/Database/Backup/Mock.hs (89 lines).
// Manifest unit: rust/glean-db/src/database/backup/mock.rs
// (migration/manifest.tsv line 61, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the filesystem backup provider used by tests and by
// the OSS default configuration. A backup is a byte-for-byte copy of the
// serialized database at
//   <site path>/<repo name>.<repo hash>
// plus a sidecar
//   <site path>/<repo name>.<repo hash>.props
// holding the backup Meta as JSON (:48-54). Restore copies the DB file into
// restoreViaFile's scratch file and then re-reads the sidecar (:62-67);
// delete removes both files (:69-72); enumerate recovers (Repo, Meta) pairs
// by scanning for *.props names (:74-84). No state is kept in the process:
// MockBackend is a unit value and MockSite is a path (:34-36).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// Three files, four rows, all followed (exact command at the repo root):
//   $ grep -n 'Backup/Mock' migration/inventory*.tsv
//
//   * migration/inventory.tsv:19 and migration/inventory.protocol.tsv:15 —
//     the strict-eager-read group row (MECHANICAL): Mock.hs:57's
//     BS.readFile is a direct std::fs::read, 1:1, no decision needed.
//     inspect below uses std::fs::read.
//   * migration/inventory.tsv:125 — the DB-admin STM_GROUP row, whose
//     evidence list includes Backup/Mock.hs: "atomically/env reads only;
//     no new state groups". This unit introduces no state (unit struct +
//     path), so the row is satisfied rather than flagged.
//   * migration/inventory.sites.tsv:440 — lazy-io at Mock.hs:57 ("lazy IO
//     ban (RULEBOOK 1)", the strict BS.readFile). Followed by the strict
//     read; no lazy handle is created.
//
// ZERO rows in migration/inventory.ffi.tsv, migration/inventory.codeflow.tsv
// and migration/inventory.stm.tsv name this file. No
// INVENTORY-BUG-CANDIDATE applies: this unit diverges from no row.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (3), all resolved:
//       edges.tsv:167  Backup/Mock.hs -> Backup/Backend.hs (manifest line
//                      56, SHIPPED: rust/glean-db/src/database/backup/
//                      backend.rs). This unit imports the traits, Data,
//                      RestoreSource, BackupError, the two closed Some
//                      enums, restore_via_file and the pub(crate)
//                      remove_path_forcibly.
//       edges.tsv:168  Backup/Mock.hs -> Glean/Repo/Text.hs (manifest line
//                      51, SHIPPED at 6ce4e79 as
//                      rust/glean-core/src/repo/text.rs):
//                      show_repo_sep(".", repo).
//       edges.tsv:169  Backup/Mock.hs -> Glean/Util/Some.hs (manifest
//                      line 38, SHIPPED at 4bdf95a as
//                      rust/glean-util/src/util/some.rs, which by design
//                      exports no code): the existentials are backend.rs's
//                      closed enums, as in the source's own Some
//                      constructors.
//     The remaining imports are base: Control.Exception (throwIO,
//     ErrorCall), Control.Monad (forM), Data.ByteString (strict),
//     Data.Maybe (catMaybes), Data.Text, System.Directory, System.FilePath,
//     System.Posix.Files, and the generated Glean.Types / Thrift.Protocol.JSON.
//     Glean.Types and hsthrift are assumed imports (RULE §4 B4), not units.
//   * INBOUND (1): edges.tsv:215 Config.hs (manifest line 85, NOT shipped)
//     imports mock and stores it under the "mock" key of cfgBackupBackends
//     (Config.hs:79, :249). Tests also import mockSite (BackupTest.hs:129,
//     DatabaseJanitorTest.hs:273); tests are re-expressed at Step 4 and are
//     not manifest units. See SIBLING-UNIT CONTRACTS.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * MockBackend/MockSite (:34-36) are the payloads of backend.rs's closed
//     SomeBackupBackend/SomeBackupSite variants, so both TYPES must be pub
//     (the enum is public). The Haskell module exports neither type, only
//     the two values (:13-16); the Rust constructor visibility is narrowed
//     where it matters: MockSite's field is private, so mock_site is its
//     only constructor, mirroring the hidden data constructor. MockBackend
//     is a unit struct and carries nothing.
//   * Some -> enum constructors. fromPath ignores its backend argument and
//     always returns Some (fromPath _ = Just . mockSite . Text.unpack,
//     :45); the Text.unpack is gone because the Rust trait already takes
//     &str (RULE §1 strings row).
//   * Every IO action -> Result<_, BackupError> (backend.rs's area error,
//     the StoreError-shaped enum). ? supplies the From<std::io::Error>
//     conversion for std::fs failures.
//   * PARTIAL FUNCTION: deserializeJSON's Left branch (:58-59) is the only
//     partial point. throwIO (ErrorCall ("can't parse props: " <> err))
//     becomes BackupError::Other(Box::new(std::io::Error::new(
//     ErrorKind::InvalidData, "can't parse props: " <> err))). This keeps
//     the catchable-exception semantics of RULE §2 (no panic) and the same
//     message prefix; the parser detail after the prefix is serde_json's,
//     not hsthrift's (DIVERGENCES 2).
//   * JSON (RULE §1 JSON row): hsthrift serializeJSON/deserializeJSON ->
//     serde_json::to_vec/from_slice on the SAME Meta serde obligation
//     pinned by database/meta.rs (its metaToProps/metaFromProps block).
//     The sidecar is the bare Meta value, NOT metaToProps' {"meta": ...}
//     string map: hsthrift serializes the struct itself.
//   * System.Posix.Files.fileSize <$> getFileStatus -> std::fs::metadata
//     (stat, i.e. following symlinks, like getFileStatus).len().
//   * System.FilePath.(</>) -> crate::database::repo::combine, the single
//     owner (repo.rs:148-162, shared with backup/backend.rs).
//   * showRepoSep "." -> glean_core::repo::text::show_repo_sep.
//   * (<.> "props") -> format!("{}.props", ..): addExtension always appends
//     ".props" even when the stem already has an extension
//     (filepath-1.4.2.2 System/FilePath/Internal.hs:298-306).
//   * takeDirectory -> take_directory: 'dropTrailingPathSeparator .
//     dropFileName' (filepath-1.4.2.2 System/FilePath/Internal.hs:523-526,
//     :552-553, :622-627, :643-644). It is spelled out over the last '/'
//     rather than delegated to std::path::Path::parent, which differs on
//     trailing separators and "." components ("a/b/" -> "a" instead of
//     "a/b").
//   * splitExtension -> split_extension, reproducing filepath-1.4.2.2's
//     last-dot split (Internal.hs:236-242 with breakEnd/spanEnd at
//     :1037-1042): the extension keeps the dot, "x." splits to ("x", "."),
//     and ".bashrc" splits to ("", ".bashrc"). read_dir yields bare names,
//     so splitFileName_'s directory component is always empty here.
//   * createDirectoryIfMissing True -> std::fs::create_dir_all.
//   * copyFile -> std::fs::copy (contents and permission bits, like
//     System.Directory.copyFile).
//   * removePathForcibly -> backend.rs's pub(crate) remove_path_forcibly
//     (one owner; missing path is success, directories recurse).
//   * listDirectory -> std::fs::read_dir, preserving the OS's iteration
//     order (Haskell's listDirectory has the same caveat).
//   * Control.Monad.forM + Data.Maybe.catMaybes (:76) -> a for loop that
//     continues on non-matching names and propagates inspect errors: the
//     source's mapM aborts enumerate on the first bad sidecar, and so does
//     the ? here.
//   * Data.Text.pack (:80, :86) -> String (RULE §1); toPath returns a clone
//     of the stored path.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. IO failures are values (BackupError::Io), not thrown IOExceptions;
//      every caller already expects Result (the backend.rs contract).
//   2. The ErrorCall payload's hsthrift parser text cannot be reproduced by
//      serde_json. The "can't parse props: " prefix is exact; the detail
//      differs. Both are diagnostic only.
//   3. serializeJSON is pure; serde_json::to_vec returns Result. The same
//      pinned handling as meta.rs's metaToProps: .expect(..), because the
//      generated Meta is plain data and a failure would be a Step-4
//      generator defect, never a runtime condition to mask.
//   4. fileSize (FileOffset = Int64) -> metadata().len() (u64) as i64. On
//      the target 64-bit platforms off_t is the same signed 64-bit value,
//      so the cast is bit-preserving for every representable size.
//   5. Non-UTF-8 directory entries: GHC's listDirectory decodes with
//      surrogate escapes and Text.pack replaces each escaped byte with
//      U+FFFD; Rust's OsString::to_string_lossy replaces each maximal
//      invalid UTF-8 subsequence with U+FFFD. The replacement COUNT can
//      therefore differ for a multi-byte invalid sequence, but neither side
//      errors, and a name that is not valid UTF-8 cannot round-trip as a
//      Repo on either side.
//   6. backup's write order is preserved: create directory, copy the DB
//      file, stat it, then write the sidecar. A failed sidecar write
//      therefore leaves the copied DB file behind, exactly as the source
//      does.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/backup/backend.rs (row 56, SHIPPED): Backend, Site, Data,
//     RestoreSource, BackupError, SomeBackupBackend, SomeBackupSite,
//     restore_via_file, and the crate-internal remove_path_forcibly. This
//     unit supplies the Mock payloads those enums forward to; the S3
//     payloads remain forward references to row 177.
//   * database/repo.rs (row 52) / glean-core's repo/text.rs (row 51):
//     combine and show_repo_sep.
//   * database/meta.rs (row 48, SHIPPED): Meta and its serde obligation;
//     the sidecar bytes are that same JSON shape.
//   * database/config.rs (row 85, future): mock() is the value stored under
//     the "mock" key (Config.hs:249); its return type is SomeBackupBackend.
//     mock_site is used by tests and by nothing in the library.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).
//
// PORT STATUS: confidence=high todos=0

use glean_core::repo::text::show_repo_sep;
use glean_thrift as thrift;

use crate::database::backup::backend::{
    remove_path_forcibly, restore_via_file, Backend, BackupError, Data, RestoreSource, Site,
    SomeBackupBackend, SomeBackupSite,
};
use crate::database::meta::Meta;
use crate::database::repo::combine;

/// 'data MockBackend = MockBackend' (:34). The payload of
/// SomeBackupBackend::Mock; carries no state.
pub struct MockBackend;

/// 'newtype MockSite = MockSite FilePath' (:36). The field is private so
/// that mock_site is the only constructor, as in the source's export list.
pub struct MockSite(String);

/// 'mock' (:38-39): the "mock" backend value (Config.hs:249).
pub fn mock() -> SomeBackupBackend {
    SomeBackupBackend::Mock(MockBackend)
}

/// 'mockSite' (:41-42): a site rooted at 'path'.
pub fn mock_site(path: &str) -> SomeBackupSite {
    SomeBackupSite::Mock(MockSite(path.to_string()))
}

impl Backend for MockBackend {
    /// 'fromPath _ = Just . mockSite . Text.unpack' (:45): every path parses
    /// as a mock site; the backend value is ignored.
    fn from_path(&self, path: &str) -> Option<SomeBackupSite> {
        Some(mock_site(path))
    }
}

impl Site for MockSite {
    /// :48-54 — copy the serialized DB to 'repo_path' and write the Meta
    /// sidecar. The TTL is not supported by this site ('_ttl').
    fn backup(
        &self,
        repo: &thrift::Repo,
        meta: &Meta,
        _ttl: Option<i64>,
        from: &str,
    ) -> Result<Data, BackupError> {
        let repo_path = repo_path(&self.0, repo);
        std::fs::create_dir_all(take_directory(&repo_path))?;
        std::fs::copy(from, &repo_path)?;
        let size = std::fs::metadata(&repo_path)?.len();

        let props = serde_json::to_vec(meta)
            .expect("Meta JSON serialization is infallible for the generated shape");
        std::fs::write(props_path(&self.0, repo), props)?;

        Ok(Data {
            data_size: size as i64,
        })
    }

    /// :56-60 — read and parse the sidecar. A missing or malformed sidecar
    /// is an error; the message prefix matches the source's ErrorCall.
    fn inspect(&self, repo: &thrift::Repo) -> Result<Meta, BackupError> {
        let bytes = std::fs::read(props_path(&self.0, repo))?;
        match serde_json::from_slice::<Meta>(&bytes) {
            Ok(meta) => Ok(meta),
            Err(err) => Err(BackupError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("can't parse props: {}", err),
            )))),
        }
    }

    /// :62-67 — copy the DB file into restoreViaFile's scratch file, then
    /// hand it to the continuation and return the re-read Meta.
    fn restore<B, F>(
        &self,
        repo: &thrift::Repo,
        scratch: &str,
        action: F,
    ) -> Result<(B, Meta), BackupError>
    where
        F: FnOnce(RestoreSource) -> Result<B, BackupError>,
    {
        restore_via_file(
            scratch,
            |file| {
                std::fs::copy(repo_path(&self.0, repo), file)?;
                self.inspect(repo)
            },
            action,
        )
    }

    /// :69-72 — remove the DB file and its sidecar. The source comments that
    /// 'backup' writes both files.
    fn delete(&self, repo: &thrift::Repo) -> Result<(), BackupError> {
        remove_path_forcibly(&repo_path(&self.0, repo))?;
        remove_path_forcibly(&props_path(&self.0, repo))?;
        Ok(())
    }

    /// :74-84 — scan the site directory for '*.props' sidecars, split each
    /// stem at its last dot into (name, hash), and read the sidecar.
    fn enumerate(&self) -> Result<Vec<(thrift::Repo, Meta)>, BackupError> {
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&self.0)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let (stem, ext) = split_extension(&file_name);
            if ext != ".props" {
                continue;
            }
            let (name, ext) = split_extension(&stem);
            // The source's pattern is (name, '.':hash); an empty hash after
            // a trailing dot ("x." -> ("x", ".")) matches, so any non-empty
            // extension starting with '.' is accepted.
            if !ext.starts_with('.') {
                continue;
            }

            let repo = thrift::Repo {
                name,
                hash: ext[1..].to_string(),
            };
            let props = self.inspect(&repo)?;
            backups.push((repo, props));
        }
        Ok(backups)
    }

    /// :86 — 'Text.pack path'.
    fn to_path(&self) -> String {
        self.0.clone()
    }
}

/// 'repoPath path repo = path </> showRepoSep "." repo' (:88-89).
fn repo_path(path: &str, repo: &thrift::Repo) -> String {
    combine(path, &show_repo_sep(".", repo))
}

/// 'repoPath path repo <.> "props"' (:53, :57, :72).
fn props_path(path: &str, repo: &thrift::Repo) -> String {
    format!("{}.props", repo_path(path, repo))
}

/// System.FilePath.takeDirectory: 'dropTrailingPathSeparator . dropFileName'
/// (filepath-1.4.2.2 System/FilePath/Internal.hs:523-526, :552-553,
/// :622-627, :643-644). dropFileName keeps the trailing separator, so the
/// result is the path through the last '/' minus trailing separators; with
/// no directory component the result is ".", and a run of separators
/// collapses to the root "/".
fn take_directory(path: &str) -> String {
    let dir = match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "",
    };
    let dir = if dir.is_empty() { "./" } else { dir };
    let stripped = dir.trim_end_matches('/');
    if stripped.is_empty() {
        "/".to_string()
    } else {
        stripped.to_string()
    }
}

/// System.FilePath.splitExtension on a bare directory entry
/// (filepath-1.4.2.2 System/FilePath/Internal.hs:236-242): split at the LAST
/// dot and keep the dot in the extension. With no dot the extension is "".
fn split_extension(name: &str) -> (String, String) {
    match name.rfind('.') {
        None => (name.to_string(), String::new()),
        Some(i) => (name[..i].to_string(), name[i..].to_string()),
    }
}
