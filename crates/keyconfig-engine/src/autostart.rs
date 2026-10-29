//! Iniciar junto com o Windows.
//!
//! - Sem admin: valor em `HKCU\...\Run`.
//! - Com admin: tarefa agendada com "privilégios mais altos", que abre elevada no
//!   logon sem mostrar o UAC. Só dá para criar ou apagar essa tarefa estando elevado.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

use crate::util::wide;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "KeyConfig";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Off,
    User,
    Admin,
}

pub fn status() -> Status {
    if task_exists() {
        Status::Admin
    } else if run_value_exists() {
        Status::User
    } else {
        Status::Off
    }
}

pub fn enable(elevated: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    if elevated {
        create_task(&exe)?;
        delete_run_value();
        Ok(())
    } else {
        set_run_value(&format!("\"{exe}\""))
    }
}

pub fn disable() -> Result<(), String> {
    delete_run_value();
    if task_exists() && !schtasks(&["/Delete", "/TN", NAME, "/F"]) {
        return Err("é preciso estar como administrador para desligar a inicialização elevada".into());
    }
    Ok(())
}

fn run_value_exists() -> bool {
    let mut len = 0;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(NAME).as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut len,
        )
    };
    result == ERROR_SUCCESS
}

fn set_run_value(command: &str) -> Result<(), String> {
    let data = wide(command);
    let result = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(NAME).as_ptr(),
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        )
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(result as i32).to_string())
    }
}

fn delete_run_value() {
    unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), wide(NAME).as_ptr()) };
}

fn task_exists() -> bool {
    schtasks(&["/Query", "/TN", NAME])
}

fn create_task(exe: &str) -> Result<(), String> {
    let user = format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default()
    );
    // O padrão do schtasks mata a tarefa após 72h, não roda na bateria e usa
    // prioridade baixa (7) — tudo ruim para um hook de teclado.
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>KeyConfig - remapeamento de teclado</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId></LogonTrigger></Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>{exe}</Command></Exec></Actions>
</Task>"#,
        user = xml_escape(&user),
        exe = xml_escape(exe),
    );

    // O schtasks lê o XML de forma confiável só em UTF-16 com BOM.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let path = std::env::temp_dir().join("keyconfig-task.xml");
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;

    let ok = schtasks(&["/Create", "/TN", NAME, "/XML", &path.to_string_lossy(), "/F"]);
    let _ = std::fs::remove_file(&path);
    if ok {
        Ok(())
    } else {
        Err("o Agendador de Tarefas recusou a criação da tarefa".into())
    }
}

fn schtasks(args: &[&str]) -> bool {
    Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .is_ok_and(|out| out.status.success())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
