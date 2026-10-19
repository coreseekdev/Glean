// Ported from: glean/util/Glean/Util/IO.hs (102 lines).
// Manifest unit: rust/glean-util/src/util/io.rs
// (migration/manifest.tsv line 63, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the filesystem helpers shared by the database and
// tests — whole-file reads (:57-58), temp-file scopes (:68-75), an atomic
// temp+rename write (:78-80), and the backup-on-error combinator (:85-102).
// The source's HandleIO class (:32-54) dispatches over five content types;
// the inventory deletes it (see EXPORT MAPPING), so the Rust surface is
// monomorphic: bytes for I/O, plus one String read helper.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// Governed rows (all followed; no INVENTORY-BUG-CANDIDATE):
//
//   * migration/inventory.tsv:111 == inventory.protocol.tsv:14
//     (LAZY_IO, confirmed), HandleIO class instances. target_translation:
//     "one Rust read(path) -> Result<Vec<u8>> (+ String helper); delete the
//     lazy instances (callers are typed by value, not instance); preserve
//     withTempFileContents/writeFileContents temp+rename atomicity".
//     Implemented exactly: read_file_contents is the bytes read,
//     read_file_contents_string the String helper, the class and all five
//     instances are gone, and write_file_contents still writes a temp file
//     in the target directory and renames it.
//   * migration/inventory.tsv:112 == inventory.protocol.tsv:33
//     (EXC_TAXONOMY, confirmed), backupFileOnError. target_translation:
//     "on_err hook on temp-file commit: any error -> copy to backup dir then
//     propagate; rename atomicity preserved; source catches even async
//     exceptions here — keep the catch-all". backup_file_on_error matches
//     every Err (not just io::Error), runs the backup, then returns the
//     original error. The backup's own failure is swallowed and logged,
//     which is what fb-util's onSomeException does (ASSUMED IMPORTS); the
//     async dimension does not exist in the Rust Result model (DIVERGENCES 6).
//   * migration/inventory.sites.tsv:469-473 — the five lazy-io sites, one per
//     getHandleContents instance (String :37, strict BS :41, lazy LBS :45,
//     strict Text :49, lazy LText :53). All five instances are deleted; the
//     two strict ones fold into std::fs::read.
//   * migration/inventory.sites.tsv:652 — exc-classify at :92, the
//     onSomeException handler; see the second row above.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * INBOUND (1): edges.tsv:181 Catalog/Local/Files.hs -> Util/IO.hs
//     (manifest line 64, NOT shipped). Files.hs:62 and :94 call
//     readFileContents on the "meta" file and pass the result to
//     Thrift.deserializeJSON, so the consumer needs the BYTES variant.
//   * OUTBOUND: none. Every import is external or base: fb-util's
//     Util.Control.Exception, Util.Log, Control.Monad.Extra, Data.ByteString
//     (strict and lazy), Data.Text (strict and lazy), System.Directory,
//     System.FilePath, System.IO, System.IO.Temp (the temporary package).
//     None has a manifest unit or a depmap edge.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * crate::log::log_error — Util.Log's logError has NO manifest unit. The
//     pinned seam shape (A10; DECISIONS 2026-10-18) is
//         pub fn log_error(msg: impl FnOnce() -> String);
//     at the crate root, rust/glean-util/src/log.rs (a future unit adds
//     'pub mod log;' to lib.rs). The first glean-util consumer,
//     util/retry_channel_exception.rs:305-330, uses a local stand-in with a
//     port marker; this unit uses the direct import, the later-unit
//     convention (database/acl_ownership.rs:137 uses glean_util::log the
//     same way). logError always forces its message (Util.Log.String.hs:32-35
//     scopes laziness to vlog, and :61-62 passes the message straight to the
//     sink), so each closure here is forced immediately.
//     Until the log unit lands, this path is an unresolved draft import,
//     like crate::internal::types in glean-db.
//   * fb-util's onSomeException (Util.Control.Exception). The OSS build pins
//     fb-util < 0.2 (glean.cabal.in:145-147, the use-folly-clib=False
//     branch); in fb-util 0.1.0.1 the definition is
//         onSomeException io f = io `catch` \exc -> do
//           withFrozenCallStack $ swallow $ f exc
//           throwIO exc
//         swallow io = void io `catchAll` \exc ->
//           logError ("swallowing exception: " ++ show exc)
//     (:115-119 and :133-135). Two consequences are preserved:
//     (a) the handler's result is discarded, and (b) an exception raised by
//     the handler is SWALLOWED and logged, never allowed to replace the
//     original. See backup_file_on_error below.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (4 exports, :9-13)
// ---------------------------------------------------------------------------
//
//   :10  HandleIO(..)        -> DELETED. The class and its five instances
//                              have no Rust item: inventory.tsv:111 deletes
//                              the lazy instances and types callers by
//                              value. read_file_contents is the bytes
//                              operation; read_file_contents_string is the
//                              row's "+ String helper".
//   :11  readFileContents    -> read_file_contents,
//                              read_file_contents_string
//   :11  writeFileContents   -> write_file_contents
//   :11  withTempFileContents-> with_temp_file_contents
//   :12  backupFileOnError   -> backup_file_on_error
//
// Private helpers: withContents (:60-65) is inlined into
// with_temp_file_contents_in (the image of the private
// withTempFileContentsIn, :72-75), which write_file_contents also calls.
// create_temp_file, open_temp_file, combine, take_directory and
// take_file_name are Rust-only helpers for System.IO.Temp and
// System.FilePath (see TRANSLATION NOTES).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * readFileContents (:57-58) = openFile ReadMode + the instance's
//     getHandleContents. The strict ByteString instance is the only one the
//     OSS consumer uses, so the Rust image is std::fs::read -> Vec<u8>. The
//     String helper reads bytes and decodes strict UTF-8 (DIVERGENCES 2).
//   * withTempFileContents (:68-70): create a uniquely named file in
//     std::env::temp_dir() with prefix "glean", write, close, call the
//     action, then remove the file on every exit path. System.IO.Temp's
//     bracket cleanup is a Drop guard here (panic-safe under unwinding).
//   * withTempFileContentsIn (:72-75): the same helper with a caller-chosen
//     directory and prefix ".glean"; private, as in the source.
//   * writeFileContents (:78-80): withTempFileContentsIn (takeDirectory
//     file) + renameFile. std::fs::rename is POSIX rename(2): atomic
//     replacement in the same directory (the temp file is created in the
//     target's directory for exactly that reason).
//   * Temp-file creation: open(2) with O_CREAT|O_EXCL and mode 0o600
//     (base's openTempFile uses 0o600), retrying on EEXIST. Names are
//     "<prefix><pid>-<counter>" from a process-global AtomicU64; only
//     uniqueness is observable, not the random suffix base generates.
//     read+write, like openTempFile's ReadWriteMode.
//   * The handle is closed (dropped) before the action runs, matching
//     withContents's hClose (:64). Rust cannot observe a close error, so a
//     failing close is unrepresentable (DIVERGENCES 4).
//   * backupFileOnError (:90-102): the Maybe dir is created first with
//     createDirectoryIfMissing False — i.e. std::fs::create_dir, one level,
//     an existing directory is fine, an existing FILE is an error — and
//     that creation is OUTSIDE the catch, so its error propagates.
//     Then the action runs; on Err the backup runs (log, createDirectoryIfMissing
//     True = create_dir_all, copyFile = std::fs::copy) and the ORIGINAL
//     error is returned. Backup failures are swallowed with
//     "swallowing exception: {e}".
//   * takeDirectory / takeFileName reproduce System.FilePath on POSIX
//     (filepath-1.4.2.2 System/FilePath/Internal.hs:534-538, :565-566,
//     :622-627, :643-644): no directory component -> ".", trailing
//     separators stripped, and a run of separators collapses to "/".
//     combine reproduces (</>) (:655-666): an absolute second argument
//     wins, empty arguments are identities, otherwise one "/" joins.
//     The system temp directory is converted with to_string_lossy
//     (FilePath -> String per RULE §1); a non-UTF-8 TMPDIR would be
//     replacement-decoded.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. HandleIO and its five instances are deleted per inventory.tsv:111.
//      Callers pass bytes; the String helper is a separate function rather
//      than a type-class dispatch. The lazy instances (String, LBS, LText)
//      are the LAZY_IO defect the row removes.
//   2. read_file_contents_string decodes strict UTF-8 and errors with
//      io::ErrorKind::InvalidData on bad input. The Haskell String instance
//      was lazy and locale-decoded (hGetContents); both the laziness and
//      the locale dependence are removed by the same inventory row. The
//      error text is Rust's, not GHC's UnicodeException text.
//   3. Temp-file names and cleanup errors: the name is deterministic
//      (prefix+pid+counter) rather than base's random suffix, and cleanup
//      errors are ignored (System.IO.Temp's ignoreIOErrors does the same).
//      Permissions match (0o600) on Unix; non-Unix builds use the process
//      umask, since mode(0o600) is a Unix extension.
//   4. A close error is unobservable in Rust: File's Drop discards it,
//      whereas hClose could throw and abort withContents. No OSS caller
//      depends on a close error.
//   5. with_temp_file_contents and backup_file_on_error require
//      E: From<std::io::Error>: the helper's own create/write failures are
//      IOExceptions, and in backup_file_on_error the pre-action create_dir
//      is outside the catch. This is the storage.rs with_materialized_file
//      shape (row 58). Box<dyn Error + Send + Sync> and DbError (which
//      gained From<io::Error> in unit 58) satisfy it. The action's own
//      error is returned UNCHANGED.
//   6. The source's catch is a SomeException catch, including async
//      exceptions. Rust's Result model has no async exception: an action
//      that panics unwinds (the temp-file guard still runs); an action that
//      returns Err takes the backup path. The "keep the catch-all" clause
//      is therefore satisfied for every representable error.
//   7. The swallow log line is "swallowing exception: {io::Error Display}".
//      The prefix is exact; the IOError rendering differs from GHC's show.
//      It is diagnostic only.
//   8. write_file_contents is POSIX-atomic (rename(2)); on a platform where
//      rename cannot replace an existing destination the write fails rather
//      than falling back to a non-atomic copy. The OSS target is Linux.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/catalog/local/files.rs (row 64, future): read_file_contents
//     returns the bytes that Thrift.deserializeJSON consumes (Files.hs:62,
//     :94). The meta-file path is built by the caller.
//   * Tests (BackupTest.hs:167, BackupTestS3.hs:56/:74) call
//     withTempFileContents with a String; tests are re-expressed at Step 4
//     and pass .as_bytes() to the bytes API.
//   * The future log owner (rust/glean-util/src/log.rs) supplies log_error;
//     this file is a consumer only.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).
//
// PORT STATUS: confidence=high todos=0

use std::io::{self, ErrorKind};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::log::log_error;

/// Process-global counter for unique temp-file names. The source's
/// System.IO.Temp uses a random suffix; only uniqueness is observable.
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// 'readFileContents' (:57-58): read the whole file as bytes.
pub fn read_file_contents(path: &str) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// The inventory's "+ String helper" (inventory.tsv:111): read the whole
/// file and decode it as strict UTF-8. The source's String instance was
/// lazy and locale-decoded; both properties are removed by the same row.
pub fn read_file_contents_string(path: &str) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    String::from_utf8(bytes).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

/// 'withTempFileContents' (:68-70): write 'contents' to a temp file under
/// the system temp directory, run 'f' on its path, and remove the file on
/// every exit path. The action's error type is E; E: From<io::Error> is
/// required for the helper's own create/write failures (the
/// with_materialized_file shape, storage.rs:225-233).
pub fn with_temp_file_contents<B, E, F>(contents: &[u8], f: F) -> Result<B, E>
where
    E: From<io::Error>,
    F: FnOnce(&str) -> Result<B, E>,
{
    let dir = std::env::temp_dir();
    with_temp_file_contents_in(&dir.to_string_lossy(), "glean", contents, f)
}

/// 'writeFileContents' (:78-80): atomically create or replace 'file' with
/// 'contents' by writing a temp file in the same directory and renaming it.
pub fn write_file_contents(file: &str, contents: &[u8]) -> io::Result<()> {
    with_temp_file_contents_in(&take_directory(file), ".glean", contents, |tmp| {
        std::fs::rename(tmp, file)
    })
}

/// 'backupFileOnError' (:85-102): run 'action' on 'filepath'; if it returns
/// an error, copy the file aside (into 'dir' or, by default, a
/// 'glean-backup-file-on-error' directory beside it), then return the
/// ORIGINAL error. A failure of the backup itself is swallowed and logged,
/// exactly like fb-util's onSomeException.
///
/// The pre-action creation of 'dir' is outside the catch in the source, so
/// its io::Error is returned as E; hence the E: From<io::Error> bound.
pub fn backup_file_on_error<B, E, F>(dir: Option<&str>, action: F, filepath: &str) -> Result<B, E>
where
    F: FnOnce(&str) -> Result<B, E>,
    E: From<io::Error>,
{
    // whenJust dir $ createDirectoryIfMissing False: one level only, an
    // existing directory is fine, an existing file is an error.
    // createDirectoryIfMissing on "" is a no-op (splitDirectories "" is
    // empty), whereas std::fs::create_dir("") fails.
    if let Some(d) = dir {
        if !d.is_empty() {
            if let Err(e) = std::fs::create_dir(d) {
                if !(e.kind() == ErrorKind::AlreadyExists && std::path::Path::new(d).is_dir()) {
                    return Err(E::from(e));
                }
            }
        }
    }

    match action(filepath) {
        Ok(result) => Ok(result),
        Err(original) => {
            // onSomeException: swallow the handler's errors, rethrow the
            // original.
            if let Err(e) = backup(dir, filepath) {
                log_error(|| format!("swallowing exception: {}", e));
            }
            Err(original)
        }
    }
}

/// 'withTempFileContentsIn' (:72-75) plus the inlined withContents (:60-65):
/// write, close, run the action, then remove the temp file.
fn with_temp_file_contents_in<B, E, F>(
    dir: &str,
    prefix: &str,
    contents: &[u8],
    f: F,
) -> Result<B, E>
where
    E: From<io::Error>,
    F: FnOnce(&str) -> Result<B, E>,
{
    use std::io::Write;

    let (path, mut file) = create_temp_file(dir, prefix)?;
    let guard = TempFileGuard { path: path.clone() };
    file.write_all(contents)?;
    // withContents's hClose runs before the action.
    drop(file);
    let result = f(&path);
    drop(guard);
    result
}

/// The 'backup' local of backupFileOnError (:94-102).
fn backup(dir: Option<&str>, filepath: &str) -> io::Result<()> {
    let subdir = match dir {
        Some(d) => d.to_string(),
        None => combine(&take_directory(filepath), "glean-backup-file-on-error"),
    };
    let dest = combine(&subdir, take_file_name(filepath));
    log_error(|| format!("Making backup of temp file: {}", dest));
    std::fs::create_dir_all(&subdir)?;
    std::fs::copy(filepath, &dest)?;
    Ok(())
}

/// Create a uniquely named temp file in 'dir'. Names use the process id and
/// a global counter; creation is O_CREAT|O_EXCL (create_new), retrying on
/// EEXIST, with mode 0o600 on Unix (base's openTempFile).
fn create_temp_file(dir: &str, prefix: &str) -> io::Result<(String, std::fs::File)> {
    for _ in 0..1000 {
        let n = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = combine(dir, &format!("{}{}-{}", prefix, std::process::id(), n));
        match open_temp_file(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        ErrorKind::AlreadyExists,
        "could not create a unique temporary file",
    ))
}

/// openTempFile's ReadWriteMode plus 0o600 on Unix.
fn open_temp_file(path: &str) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Remove the temp file on drop; a file the action renamed is already gone
/// (NotFound is success, and every cleanup error is ignored, like
/// System.IO.Temp's ignoreIOErrors).
struct TempFileGuard {
    path: String,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(_) => {}
        }
    }
}

/// System.FilePath's (</>) on POSIX (filepath-1.4.2.2
/// System/FilePath/Internal.hs:655-666): an absolute second argument wins;
/// empty arguments are identities; otherwise one "/" joins.
fn combine(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    if b.starts_with('/') {
        return b.to_string();
    }
    if a.ends_with('/') {
        return format!("{}{}", a, b);
    }
    format!("{}/{}", a, b)
}

/// System.FilePath.takeDirectory (filepath-1.4.2.2
/// System/FilePath/Internal.hs:622-627, :643-644): no directory component
/// means ".", trailing separators are stripped, and a run of separators
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

/// System.FilePath.takeFileName (filepath-1.4.2.2
/// System/FilePath/Internal.hs:534-538, :565-566): everything after the last
/// path separator (the whole string when there is none).
fn take_file_name(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}
