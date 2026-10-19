// Ported from: glean/util/Glean/Util/Disk.hs (37 lines).
// Manifest unit: rust/glean-util/src/util/disk.rs
// (migration/manifest.tsv line 69, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: three one-line disk-capacity queries — total size, used
// bytes, available bytes for the filesystem holding a path. All three are
// getDfOutput with a different --output= field (:19-30), and getDfOutput
// shells out to `df --output=<field> -B1 '<path>'` and parses the second
// whitespace-separated word of stdout (:33-37). The header line df prints
// (`1B-blocks`, `Used`, `Avail`) is what word 0 is; word 1 is the byte
// count.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file. Exact commands at the repo root:
//   $ grep -n 'Util/Disk' migration/inventory*.tsv
//   $ grep -n 'Disk.hs' migration/inventory*.tsv
//   (no output; exit status 1 for both)
// A construct-level search is likewise empty:
//   $ grep -n 'readCreateProcess\|System.Process' migration/inventory*.tsv
//   (no output)
// so the shell-out itself carries no inventory decision; the exception
// mapping below is this file's own call under RULE §2's dynamic-exceptions
// row (typed error per crate — here std::io::Error, the crate's existing
// convention, util/io.rs). No inventory divergence applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. The source's only imports are System.Process (:14)
//     and Text.Printf (:15), both boot packages with no manifest unit and
//     no depmap edge.
//   * INBOUND (2), both future units:
//       edges.tsv:429 Storage/LMDB.hs (manifest line 70, NOT shipped) imports
//         at LMDB.hs:41 and calls getDiskSize/getUsedDiskSpace/
//         getFreeDiskSpace at :114/:123/:131.
//       edges.tsv:444 Storage/RocksDB.hs (manifest line 72, NOT shipped)
//         imports at RocksDB.hs:48 and calls them at :153/:162/:170.
//     glean/test/tests/StorageRocksDBTest.hs:22,33,71 is the third caller but
//     tests are not translated (RULE §4 tests row).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * NONE — std::io and std::process are std; nothing generated or
//     cross-crate is involved.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * FilePath -> IO Int (:19, :24, :29) -> &str -> io::Result<i64>.
//     FilePath is the String synonym, so the crate's &str-in convention
//     applies (database/backup/backend.rs:97); GHC Int is 64-bit, the same
//     mapping as util/service.rs:139. IO keeps a Result because the source
//     documents and throws an IOException on a missing path (:18, :23, :28).
//   * getDfOutput :: String -> FilePath -> IO Int (:33) stays private
//     (get_df_output), argument order preserved.
//   * The command string is reproduced verbatim:
//       printf "df --output=%s -B1 '%s'" outp path     (:37)
//     i.e. format!("df --output={} -B1 '{}'", outp, path) — no escaping,
//     see the source-defect note below.
//   * shell (:37) -> /bin/sh -c <cmd>: commandToProcess (ShellCommand s) =
//     ("/bin/sh", ["-c", s]) (process-1.6.17.0, the version the tree
//     builds, System/Process/Posix.hs). No $SHELL, no Windows branch.
//   * readCreateProcess cmd "" (:35) -> spawn with stdin and stdout piped
//     and stderr INHERITED (readCreateProcess sets only std_in and std_out;
//     stderr stays Inherit), write the empty input (the source's
//     `unless (null input)` writes nothing), close stdin, read stdout to
//     EOF, then wait. Its stdout is decoded as UTF-8; the source uses the
//     locale encoding via hGetContents, which throws on invalid bytes —
//     hence ErrorKind::InvalidData rather than a lossy conversion.
//   * A non-zero exit is an IOException in the source:
//       processFailedException "readCreateProcess" cmd [] r
//     = ioError (mkIOError OtherError "readCreateProcess: <cmd> (exit <r>)")
//     (process-1.6.17.0 System/Process.hs). The text users see is
//     `show (exc :: SomeException)`, which renders the whole IOError:
//     "<ioe_location>: failed", because ioe_type is OtherError (base
//     4.17.1.0 GHC/IO/Exception.hs:394, :417-430). The port therefore
//     reproduces the trailing ": failed" as well — verified against the
//     pinned GHC 9.4.7: `callProcess "false" []` shows
//     `callProcess: false (exit 1): failed`. A signal-killed child has no
//     exit code, and GHC reports ExitFailure (-signal), so the negated
//     signal is used there.
//   * words (:35) -> split_whitespace (both split on Unicode whitespace and
//     drop empty fields; df's numeric column is ASCII, and a translated
//     header is still one word).
//   * (!! 1) (:35) is partial: fewer than two words panics with GHC's
//     message "Prelude.!!: index too large".
//   * read :: String -> Int (:35) is partial: a non-number panics with
//     GHC's "Prelude.read: no parse". A leading '+' is NOT a number to
//     Haskell's lexer (only '-' is), so it is rejected explicitly before
//     str::parse, which would otherwise accept it.
//
// ---------------------------------------------------------------------------
// SOURCE DEFECT (reproduced)
// ---------------------------------------------------------------------------
//
// BUG(port): the path is interpolated into a single-quoted shell word with no
// escaping (:37). A path containing ' therefore breaks the command, and a
// crafted path injects shell commands: /tmp/a'; id; echo ' runs id.
// Repro: get_disk_size("/tmp/a'b") sends
// `df --output=size -B1 '/tmp/a'b'` to /bin/sh, which exits 2 with
// "Syntax error: Unterminated quoted string", so the call returns
// `readCreateProcess: df --output=size -B1 '/tmp/a'b' (exit 2)`.
// Reproduced verbatim (no escaping added): RULE §2's bug rule, fixes only as
// flagged post-parity changes.
//
// ---------------------------------------------------------------------------
// DIVERGENCES
// ---------------------------------------------------------------------------
//
//   1. read on an out-of-range literal wraps via fromInteger (Int is a
//      ring); str::parse::<i64> fails instead and the port panics with
//      "Prelude.read: no parse". Unreachable for df output (a filesystem
//      cannot report more than 2^63 bytes in this field), so the simpler
//      total-parse path is kept rather than emulating the wrap. Haskell's
//      Read Int also accepts forms str::parse rejects — hex/octal literals
//      ("0x10"), a parenthesised "(5)", a sign followed by a space
//      ("- 5") — so those panic here where the source would return a
//      number; df prints none.
//   2. words uses Data.Char.isSpace; split_whitespace uses
//      char::is_whitespace. The two tables differ only for exotic
//      non-ASCII code points that df never emits.
//   3. The source forces stdout before waiting for the exit code; the port
//      reads to EOF inside wait_with_output, which is the same order.
//   4. std::os::unix::process::ExitStatusExt is imported unconditionally.
//      The module is POSIX-only by construction: the source shells out to
//      df, which has no Windows equivalent, so the build target is already
//      unix.
//   5. Spawn-time failures bypass the readCreateProcess text: a NUL byte
//      is rejected by Command::spawn (InvalidInput) where the C shell
//      string would truncate at the NUL, and an over-long argument list
//      gives E2BIG ("Argument list too long") where GHC says
//      "createProcess: posix_spawnp: resource exhausted". POSIX paths
//      cannot contain NUL and are far below ARG_MAX, so both are
//      unreachable.
//   6. The source's fmap over readCreateProcess leaves the (!! 1)/read
//      errors to fire on first FORCE of the result (A6 timing); the port
//      panics inside the call. Every reachable consumer forces the value:
//      getTotalCapacity (LMDB.hs:114-116, RocksDB.hs:153-155),
//      getFreeCapacity (:131, :170), and getUsedCapacity (:123, :162)
//      returns it unforced but its only caller subtracts it. Nothing
//      reachable differs.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 70 (storage/lmdb.rs) and row 72 (storage/rocks_db.rs) call these
//     from their capacity methods:
//       getTotalCapacity (LMDB.hs:110-118, RocksDB.hs:149-157) takes
//         getDiskSize and mins it against the virtual limit;
//       getUsedCapacity (:120-124, :159-163) is
//         `Just <$> getUsedDiskSpace` with no limit check;
//       getFreeCapacity (:126-131, :165-170) subtracts used from total and
//         falls back to getFreeDiskSpace when either side is Nothing.
//     getTotalCapacity and getUsedCapacity are guarded by
//     doesDirectoryExist, so a missing root yields None there. The
//     getFreeCapacity fallback is NOT guarded, and Storage.hs:160 types it
//     `IO Int` (not Maybe), so a missing root surfaces the IOException from
//     getFreeDiskSpace at that call. Neither storage catches around these:
//     the only IOException handler in either file (LMDB.hs:158) wraps the
//     `umount` in close. The i64 return is what the min/subtract expect.
//
// MARKERS: one source-defect reproduction (the unescaped shell quoting), zero
// port todos, zero performance deferrals, zero inventory divergences, zero
// SAFETY notes (no unsafe code).
//
// PORT STATUS: confidence=high todos=0

use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

/// `getDiskSize :: FilePath -> IO Int` (:19-20): the size in bytes of the
/// volume holding `path`. Throws (returns Err) when the path does not exist,
/// because `df` then exits non-zero.
pub fn get_disk_size(path: &str) -> io::Result<i64> {
    get_df_output("size", path)
}

/// `getUsedDiskSpace :: FilePath -> IO Int` (:24-25): the used bytes.
/// Same error contract as `get_disk_size` (:23).
pub fn get_used_disk_space(path: &str) -> io::Result<i64> {
    get_df_output("used", path)
}

/// `getFreeDiskSpace :: FilePath -> IO Int` (:29-30): the available bytes.
/// Same error contract as `get_disk_size` (:28).
pub fn get_free_disk_space(path: &str) -> io::Result<i64> {
    get_df_output("avail", path)
}

/// `getDfOutput :: String -> FilePath -> IO Int` (:33-37): run `df` for one
/// column and read the second word of its output. Throws (returns Err) when
/// the path does not exist (:32).
fn get_df_output(outp: &str, path: &str) -> io::Result<i64> {
    // cmd = shell (printf "df --output=%s -B1 '%s'" outp path) (:37). The
    // path is deliberately NOT escaped — see the source-defect note.
    let cmd = format!("df --output={} -B1 '{}'", outp, path);
    let stdout = read_create_process(&cmd)?;
    // read . (!! 1) . words (:35).
    let words: Vec<&str> = stdout.split_whitespace().collect();
    let field = match words.get(1) {
        Some(field) => *field,
        // GHC's (!!): "Prelude.!!: index too large".
        None => panic!("Prelude.!!: index too large"),
    };
    // Haskell's Read Int rejects a leading '+' (the lexer only allows a
    // leading '-'); str::parse would accept it.
    if field.starts_with('+') {
        panic!("Prelude.read: no parse");
    }
    match field.parse::<i64>() {
        Ok(n) => Ok(n),
        // GHC's read: "Prelude.read: no parse".
        Err(_) => panic!("Prelude.read: no parse"),
    }
}

/// `readCreateProcess cmd ""` (:35): run `cmd` through `/bin/sh -c`, with
/// stderr inherited, and return stdout. A non-zero exit becomes the source's
/// `readCreateProcess: <cmd> (exit <n>): failed` IOException (the suffix
/// is Show's rendering of ioe_type = OtherError).
fn read_create_process(cmd: &str) -> io::Result<String> {
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    // `unless (null input) $ hPutStr inh input` writes nothing for ""; the
    // take+drop is hClose inh.
    drop(child.stdin.take());
    let output = child.wait_with_output()?;
    // readCreateProcess forces (decodes) the output BEFORE waitForProcess
    // checks the exit code, so a decode error wins over a non-zero exit.
    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if !output.status.success() {
        let code = match output.status.code() {
            Some(code) => code,
            // Killed by a signal: GHC's ExitCode is ExitFailure (-signal).
            None => -output.status.signal().unwrap_or(0),
        };
        return Err(io::Error::other(format!(
            "readCreateProcess: {} (exit {}): failed",
            cmd, code
        )));
    }
    Ok(stdout)
}
