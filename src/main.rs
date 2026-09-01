/*
PC Diagnostics Interactive CLI Tool
Features:
- Clean, high-contrast box-drawing layouts with clear visual hierarchy.
- Screen clearing on each menu action and test run.
- Crash-safe process listing sorting (handling NaN floating-point edge cases).
- Debug Mode inspecting executable path, architecture, thread counts, memory limits, and WMI connectivity.
- Loops cleanly until explicit exit.
*/

use colored::*;
use serde::Deserialize;
use std::cmp::Ordering;
use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_BaseBoard")]
struct Win32BaseBoard {
    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,
    #[serde(rename = "Product")]
    product: Option<String>,
    #[serde(rename = "SerialNumber")]
    serial_number: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_VideoController")]
struct Win32VideoController {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "DriverVersion")]
    driver_version: Option<String>,
    #[serde(rename = "AdapterRAM")]
    adapter_ram: Option<u64>,
    #[serde(rename = "VideoProcessor")]
    video_processor: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_CDROMDrive")]
struct Win32CDROMDrive {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Drive")]
    drive: Option<String>,
    #[serde(rename = "MediaLoaded")]
    media_loaded: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
struct Win32DiskDrive {
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "InterfaceType")]
    interface_type: Option<String>,
    #[serde(rename = "MediaType")]
    media_type: Option<String>,
    #[serde(rename = "Size")]
    size: Option<u64>,
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

fn render_bar(percentage: f32, width: usize) -> String {
    let clamped = percentage.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar_str = format!("[{}{}]", "■".repeat(filled), " ".repeat(empty));

    if clamped < 60.0 {
        bar_str.green().to_string()
    } else if clamped < 85.0 {
        bar_str.yellow().to_string()
    } else {
        bar_str.red().to_string()
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn print_section_header(title: &str) {
    println!("\n  ┌{}┐", "─".repeat(66));
    println!("  │  {:<64}│", title.bold().cyan());
    println!("  └{}┘", "─".repeat(66));
}

fn show_system() {
    print_section_header("SYSTEM & MOTHERBOARD");

    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let minutes = (uptime % 3600) / 60;
    let seconds = uptime % 60;

    println!("  {:<24} : {}", "Operating System", System::name().unwrap_or_default().bold());
    println!("  {:<24} : {}", "OS Version", System::os_version().unwrap_or_default());
    println!("  {:<24} : {}", "Kernel Build", System::kernel_version().unwrap_or_default());
    println!("  {:<24} : {}", "Host Machine Name", System::host_name().unwrap_or_default().yellow());
    println!("  {:<24} : {}d {}h {}m {}s", "System Uptime", days, hours, minutes, seconds);

    if let Ok(com) = COMLibrary::new() {
        if let Ok(wmi) = WMIConnection::new(com) {
            if let Ok(boards) = wmi.query::<Win32BaseBoard>() {
                for board in boards {
                    println!("  {:<24} : {}", "Board Manufacturer", board.manufacturer.unwrap_or_default().trim());
                    println!("  {:<24} : {}", "Board Product / Model", board.product.unwrap_or_default().trim().bold());
                    println!("  {:<24} : {}", "Board Serial Number", board.serial_number.unwrap_or_default().trim());
                }
            }
        }
    }
}

fn show_cpu(sys: &System) {
    print_section_header("PROCESSOR (CPU) DIAGNOSTICS");

    let cpus = sys.cpus();
    if let Some(first) = cpus.first() {
        println!("  {:<20} : {}", "Processor Model", first.brand().trim().bold());
        println!("  {:<20} : {} MHz", "Clock Speed", first.frequency());
        println!("  {:<20} : {} Cores / Threads", "Logical Compute", cpus.len());
    }

    let global_load = sys.global_cpu_usage();
    println!("  {:<20} : {:>5.1}%  {}", "Total CPU Load", global_load, render_bar(global_load, 24));

    println!("\n  {}", "Core Utilization Breakdown:".bold().white());
    for (idx, cpu) in cpus.iter().enumerate() {
        let load = cpu.cpu_usage();
        println!("    Core #{:<2}  [{:>5.1}%]  {}", idx, load, render_bar(load, 18));
    }
}

fn show_memory(sys: &System) {
    print_section_header("MEMORY STATUS (RAM & PAGE FILE)");

    let total_ram = sys.total_memory();
    let used_ram = sys.used_memory();
    let free_ram = sys.free_memory();
    let ram_percent = if total_ram > 0 { (used_ram as f32 / total_ram as f32) * 100.0 } else { 0.0 };

    println!("  {:<22} : {}", "Total Physical Memory", format_bytes(total_ram).bold());
    println!("  {:<22} : {}", "Used Physical Memory", format_bytes(used_ram).yellow());
    println!("  {:<22} : {}", "Available Free Memory", format_bytes(free_ram).green());
    println!("  {:<22} : {:>5.1}%  {}", "RAM Utilization", ram_percent, render_bar(ram_percent, 24));

    let total_swap = sys.total_swap();
    let used_swap = sys.used_swap();
    if total_swap > 0 {
        let swap_percent = (used_swap as f32 / total_swap as f32) * 100.0;
        println!("\n  {:<22} : {}", "Page File Capacity", format_bytes(total_swap));
        println!("  {:<22} : {}", "Used Page File", format_bytes(used_swap));
        println!("  {:<22} : {:>5.1}%  {}", "Page File Load", swap_percent, render_bar(swap_percent, 24));
    }
}

fn show_gpu() {
    print_section_header("GRAPHICS & DISPLAY CONTROLLERS");

    let mut found = false;
    if let Ok(com) = COMLibrary::new() {
        if let Ok(wmi) = WMIConnection::new(com) {
            if let Ok(gpus) = wmi.query::<Win32VideoController>() {
                for (i, gpu) in gpus.iter().enumerate() {
                    found = true;
                    let name = gpu.name.clone().unwrap_or_else(|| "Generic Display Controller".to_string());
                    let driver = gpu.driver_version.clone().unwrap_or_else(|| "Unknown".to_string());
                    let ram = gpu.adapter_ram.map(format_bytes).unwrap_or_else(|| "Shared / Dynamic".to_string());
                    let proc = gpu.video_processor.clone().unwrap_or_else(|| "Integrated / N/A".to_string());

                    println!("  [Adapter #{}] {}", i + 1, name.bold().magenta());
                    println!("    ├─ {:<16} : {}", "Driver Version", driver);
                    println!("    ├─ {:<16} : {}", "Dedicated VRAM", ram);
                    println!("    └─ {:<16} : {}", "Processor Type", proc);
                }
            }
        }
    }

    if !found {
        println!("  No WMI Display Controllers found.");
    }
}

fn show_storage() {
    print_section_header("STORAGE DRIVES & MOUNTED PARTITIONS");

    let disks = Disks::new_with_refreshed_list();
    println!("  {}", "Mounted File Systems:".bold().white());
    for disk in &disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let usage_pct = if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 };

        println!("    Volume {} [{}]", disk.mount_point().to_string_lossy().yellow().bold(), disk.file_system().to_string_lossy());
        println!("      ├─ Total: {:<10} Free: {:<10} Used: {}", format_bytes(total), format_bytes(available), format_bytes(used));
        println!("      └─ Usage: {:>5.1}% {}", usage_pct, render_bar(usage_pct, 20));
    }

    if let Ok(com) = COMLibrary::new() {
        if let Ok(wmi) = WMIConnection::new(com) {
            println!("\n  {}", "Physical Hardware Drives (WMI):".bold().white());
            if let Ok(drives) = wmi.query::<Win32DiskDrive>() {
                for drive in drives {
                    let model = drive.model.unwrap_or_default().trim().to_string();
                    let iface = drive.interface_type.unwrap_or_default().trim().to_string();
                    let media = drive.media_type.unwrap_or_default().trim().to_string();
                    let size = drive.size.map(format_bytes).unwrap_or_default();
                    println!("    • {:<32} | Bus: {:<6} | Type: {:<12} | Capacity: {}", model.bold(), iface.cyan(), media, size);
                }
            }

            println!("\n  {}", "Optical / CD-ROM Units:".bold().white());
            if let Ok(cdroms) = wmi.query::<Win32CDROMDrive>() {
                if cdroms.is_empty() {
                    println!("    None detected.");
                }
                for cd in cdroms {
                    println!("    • Drive {} ({}) - Media Ready: {}", 
                        cd.drive.unwrap_or_default().yellow(),
                        cd.name.unwrap_or_default().trim(),
                        cd.media_loaded.unwrap_or(false)
                    );
                }
            }
        }
    }
}

fn show_network() {
    print_section_header("NETWORK INTERFACES & TRAFFIC");

    let networks = Networks::new_with_refreshed_list();
    for (name, data) in &networks {
        println!("  Interface: {}", name.bold().blue());
        println!("    ├─ {:<16} : {}", "Physical (MAC)", data.mac_address());
        println!("    ├─ {:<16} : {}", "Total Received", format_bytes(data.total_received()).green());
        println!("    └─ {:<16} : {}", "Total Sent", format_bytes(data.total_transmitted()).yellow());
    }
}

fn show_tasks(sys: &System) {
    print_section_header("ACTIVE PROCESSES (TOP 15 BY CPU)");

    let mut processes: Vec<_> = sys.processes().values().collect();

    // Safe float comparison to prevent crash on NaN values
    processes.sort_by(|a, b| {
        b.cpu_usage()
            .partial_cmp(&a.cpu_usage())
            .unwrap_or(Ordering::Equal)
    });

    println!("  ┌{0:─<8}┬{0:─<32}┬{0:─<12}┬{0:─<14}┐", "");
    println!("  │ {:<6} │ {:<30} │ {:<10} │ {:<12} │", "PID", "PROCESS NAME", "CPU %", "MEMORY");
    println!("  ├{0:─<8}┼{0:─<32}┼{0:─<12}┼{0:─<14}┤", "");

    for proc in processes.iter().take(15) {
        let name = proc.name().to_string_lossy();
        let display_name = if name.len() > 30 { format!("{}...", &name[..27]) } else { name.to_string() };

        println!("  │ {:<6} │ {:<30} │ {:>8.1}%  │ {:>12} │", 
            proc.pid().to_string().cyan(),
            display_name,
            proc.cpu_usage(),
            format_bytes(proc.memory())
        );
    }
    println!("  └{0:─<8}┴{0:─<32}┴{0:─<12}┴{0:─<14}┘", "");
}

fn show_debug(sys: &System) {
    print_section_header("DIAGNOSTIC TOOL DEBUG CONTEXT");

    let start = Instant::now();
    let current_exe = env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "Unknown".to_string());
    let current_dir = env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| "Unknown".to_string());

    println!("  {:<26} : {}", "Binary Location", current_exe);
    println!("  {:<26} : {}", "Execution Directory", current_dir);
    println!("  {:<26} : {}", "Target Architecture", env::consts::ARCH);
    println!("  {:<26} : {}", "Host Family", env::consts::FAMILY);
    println!("  {:<26} : {}", "Detected CPU Threads", sys.cpus().len());
    println!("  {:<26} : {}", "Tracked Processes Count", sys.processes().len());
    println!("  {:<26} : {}", "Tracked Disks Count", Disks::new_with_refreshed_list().len());
    println!("  {:<26} : {}", "Tracked Network Adapters", Networks::new_with_refreshed_list().len());

    let wmi_status = match COMLibrary::new() {
        Ok(com) => match WMIConnection::new(com) {
            Ok(_) => "Connected & Initialized (OK)".green().to_string(),
            Err(e) => format!("Connection Failed: {}", e).red().to_string(),
        },
        Err(e) => format!("COM Init Failed: {}", e).red().to_string(),
    };
    println!("  {:<26} : {}", "WMI / COM Status", wmi_status);

    let duration = start.elapsed();
    println!("  {:<26} : {:?}", "Debug Probe Latency", duration);
}

fn display_menu() {
    clear_screen();
    println!("  ╔{}╗", "═".repeat(50));
    println!("  ║           {}           ║", "PC HARDWARE & SYSTEM DIAGNOSTICS".bold().white());
    println!("  ╠{}╣", "═".repeat(50));
    println!("  ║  [{}]  Run Full System Diagnostics (All)        ║", "1".green().bold());
    println!("  ║  [{}]  System, OS & Motherboard                 ║", "2".green().bold());
    println!("  ║  [{}]  Processor (CPU) Status                   ║", "3".green().bold());
    println!("  ║  [{}]  Memory (RAM & Swap) Status               ║", "4".green().bold());
    println!("  ║  [{}]  Graphics (GPU & Display Drivers)         ║", "5".green().bold());
    println!("  ║  [{}]  Storage (Drives, Partitions, CD)         ║", "6".green().bold());
    println!("  ║  [{}]  Network Adapters & Traffic               ║", "7".green().bold());
    println!("  ║  [{}]  Active Tasks (Processes by CPU)          ║", "8".green().bold());
    println!("  ║  [{}]  Debug Mode (Diagnostics Context)         ║", "9".yellow().bold());
    println!("  ║  [{}]  Exit                                     ║", "0".red().bold());
    println!("  ╚{}╝", "═".repeat(50));
    print!("  Select an option [0-9]: ");
    let _ = io::stdout().flush();
}

fn refresh_system(sys: &mut System) {
    thread::sleep(Duration::from_millis(300));
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
}

fn main() {
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(sysinfo::ProcessRefreshKind::everything()),
    );

    loop {
        display_menu();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let choice = input.trim();
        refresh_system(&mut sys);
        clear_screen();

        match choice {
            "1" => {
                show_system();
                show_cpu(&sys);
                show_memory(&sys);
                show_gpu();
                show_storage();
                show_network();
                show_tasks(&sys);
            }
            "2" => show_system(),
            "3" => show_cpu(&sys),
            "4" => show_memory(&sys),
            "5" => show_gpu(),
            "6" => show_storage(),
            "7" => show_network(),
            "8" => show_tasks(&sys),
            "9" => show_debug(&sys),
            "0" | "exit" | "q" => {
                println!("\n  Exiting diagnostics.\n");
                break;
            }
            _ => {
                println!("\n  {}", "Invalid selection. Please choose a number from 0 to 9.".red());
            }
        }

        print!("\n  Press Enter to return to menu...");
        let _ = io::stdout().flush();
        let mut buffer = String::new();
        let _ = io::stdin().read_line(&mut buffer);
    }
}