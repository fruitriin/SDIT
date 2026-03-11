use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    process::ExitStatus,
};

use thiserror::Error;

/// PTY 関連エラー
#[derive(Debug, Error)]
pub enum PtyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("pty-process error: {0}")]
    PtyProcess(#[from] pty_process::Error),
    #[error("shell not found")]
    ShellNotFound,
}

pub type Result<T> = std::result::Result<T, PtyError>;

/// PTY のターミナルサイズ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl PtySize {
    #[must_use]
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { rows, cols, pixel_width: 0, pixel_height: 0 }
    }
}

impl Default for PtySize {
    fn default() -> Self {
        Self::new(24, 80)
    }
}

/// PTY 起動設定
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// 起動するシェル。None の場合は `$SHELL` または `/bin/sh`
    pub shell: Option<String>,
    /// シェルへ渡す追加引数
    pub args: Vec<String>,
    /// 作業ディレクトリ
    pub working_directory: Option<PathBuf>,
    /// 追加の環境変数
    pub env: HashMap<String, String>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        let shell = std::env::var("SHELL").ok();
        Self { shell, args: Vec::new(), working_directory: None, env: HashMap::new() }
    }
}

/// PTY（擬似端末）と子プロセスの管理構造体
pub struct Pty {
    pty: pty_process::blocking::Pty,
    child: std::process::Child,
}

impl Pty {
    /// PTY の master fd をクローンして書き込み専用の `File` を返す。
    ///
    /// read スレッドと write スレッドを分離するために使用する。
    /// クローンされた fd は同じ PTY master を指すため、
    /// 一方で read、もう一方で write を同時に行える。
    ///
    /// # Errors
    /// fd のクローンに失敗した場合にエラーを返す。
    pub fn try_clone_writer(&self) -> Result<std::fs::File> {
        use std::os::fd::AsFd;
        let fd = self.pty.as_fd().try_clone_to_owned().map_err(PtyError::Io)?;
        Ok(std::fs::File::from(fd))
    }

    /// 新しい PTY を生成してシェルを起動する
    ///
    /// # Errors
    /// PTY の生成・サイズ設定・プロセス起動に失敗した場合にエラーを返す。
    pub fn spawn(config: &PtyConfig, size: PtySize) -> Result<Self> {
        let shell = config.shell.as_deref().filter(|s| !s.is_empty()).unwrap_or("/bin/sh");

        let pty = pty_process::blocking::Pty::new()?;
        let pts = pty.pts()?;
        let mut cmd = pty_process::blocking::Command::new(shell);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        if let Some(dir) = &config.working_directory {
            cmd.current_dir(dir);
        }
        let child = cmd.spawn(&pts)?;

        // macOS 26+ では spawn 前の TIOCSWINSZ が ENOTTY になるため、spawn 後に resize する
        pty.resize(pty_process::Size::new(size.rows, size.cols))?;

        Ok(Self { pty, child })
    }

    /// ターミナルサイズを変更する
    ///
    /// # Errors
    /// ioctl の呼び出しに失敗した場合にエラーを返す。
    pub fn resize(&self, size: PtySize) -> Result<()> {
        self.pty.resize(pty_process::Size::new(size.rows, size.cols)).map_err(PtyError::PtyProcess)
    }

    /// PTY の master fd をクローンしてリサイズ専用の `OwnedFd` を返す。
    ///
    /// Session が Pty を Reader スレッドに move した後でも、
    /// この fd を使って `TIOCSWINSZ` ioctl を呼び子プロセスに SIGWINCH を送れる。
    ///
    /// # Errors
    /// fd のクローンに失敗した場合にエラーを返す。
    pub fn try_clone_resize_fd(&self) -> Result<std::os::fd::OwnedFd> {
        use std::os::fd::AsFd;
        self.pty.as_fd().try_clone_to_owned().map_err(PtyError::Io)
    }

    /// 子プロセスの PID を返す。
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    /// 子プロセスが終了しているか確認する（ノンブロッキング）
    ///
    /// # Errors
    /// OS の呼び出しに失敗した場合にエラーを返す。
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().map_err(PtyError::Io)
    }

    /// 子プロセスを強制終了する
    ///
    /// # Errors
    /// kill の呼び出しに失敗した場合にエラーを返す。
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().map_err(PtyError::Io)
    }
}

impl Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.pty.read(buf)
    }
}

impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pty.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.pty.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_pty_config(shell: &str, args: &[&str]) -> PtyConfig {
        PtyConfig {
            shell: Some(shell.to_owned()),
            args: args.iter().map(|s| (*s).to_owned()).collect(),
            working_directory: None,
            env: HashMap::new(),
        }
    }

    // headless/CI では resize ioctl が ENOTTY で失敗するため TTY 判定でスキップ
    fn is_tty() -> bool {
        std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty").is_ok()
    }

    #[test]
    fn test_pty_size_default() {
        let size = PtySize::default();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn test_pty_size_new() {
        let size = PtySize::new(40, 120);
        assert_eq!(size.rows, 40);
        assert_eq!(size.cols, 120);
    }

    #[test]
    fn test_pty_config_default_shell() {
        let config = PtyConfig::default();
        // $SHELL が設定されている場合はその値、なければ None
        if let Ok(shell) = std::env::var("SHELL") {
            assert_eq!(config.shell, Some(shell));
        } else {
            assert_eq!(config.shell, None);
        }
    }

    #[test]
    fn test_pty_spawn_and_echo() {
        if !is_tty() {
            eprintln!("skipping PTY test: not a TTY environment");
            return;
        }

        let config = make_pty_config("echo", &["hello"]);
        let size = PtySize::new(24, 80);
        let mut pty = Pty::spawn(&config, size).expect("PTY spawn failed");

        let mut output = Vec::new();
        let mut buf = [0u8; 256];
        let deadline = std::time::Instant::now() + Duration::from_secs(5);

        loop {
            match pty.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    output.extend_from_slice(&buf[..n]);
                    if output.windows(5).any(|w| w == b"hello") {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                // EIO (errno 5): PTY が閉じられた（子プロセス終了）
                Err(e) if e.raw_os_error() == Some(5) => break,
                Err(_) => break,
            }
        }

        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("hello"), "expected 'hello' in PTY output, got: {text:?}");

        let _ = pty.try_wait();
    }

    #[test]
    fn test_pty_spawn_shell() {
        if !is_tty() {
            eprintln!("skipping PTY test: not a TTY environment");
            return;
        }

        let config = make_pty_config("/bin/sh", &[]);
        let size = PtySize::new(24, 80);
        let mut pty = Pty::spawn(&config, size).expect("PTY spawn failed");

        // spawn 直後は子プロセスがまだ生存していること
        let status = pty.try_wait().expect("try_wait failed");
        assert!(status.is_none(), "child should still be running after spawn");

        pty.kill().expect("kill failed");
    }

    #[test]
    fn test_pty_resize() {
        if !is_tty() {
            eprintln!("skipping PTY test: not a TTY environment");
            return;
        }

        let config = make_pty_config("/bin/sh", &[]);
        let size = PtySize::new(24, 80);
        let mut pty = Pty::spawn(&config, size).expect("PTY spawn failed");

        // resize が成功すること（エラーなし）
        pty.resize(PtySize::new(40, 120)).expect("resize failed");

        // resize 後も子プロセスが生存していること
        let status = pty.try_wait().expect("try_wait failed");
        assert!(status.is_none(), "child should still be running after resize");

        pty.kill().expect("kill failed");
    }

    #[test]
    fn test_pty_write_and_read() {
        if !is_tty() {
            eprintln!("skipping PTY test: not a TTY environment");
            return;
        }

        let config = make_pty_config("/bin/sh", &[]);
        let size = PtySize::new(24, 80);
        let mut pty = Pty::spawn(&config, size).expect("PTY spawn failed");

        // シェルに echo コマンドを送信
        pty.write_all(b"echo world\n").expect("write failed");

        let mut output = Vec::new();
        let mut buf = [0u8; 512];
        let deadline = std::time::Instant::now() + Duration::from_secs(5);

        loop {
            match pty.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    output.extend_from_slice(&buf[..n]);
                    if output.windows(5).any(|w| w == b"world") {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) if e.raw_os_error() == Some(5) => break,
                Err(_) => break,
            }
        }

        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("world"), "expected 'world' in PTY output, got: {text:?}");

        let _ = pty.kill();
    }
}
