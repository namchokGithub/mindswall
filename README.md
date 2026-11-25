# MindsWall

โปรแกรมจัดการวอลเปเปอร์สำหรับ Windows ที่ออกแบบให้มีน้ำหนักเบาและทำงานแบบ local-first โดยไม่จำเป็นต้องเชื่อมต่ออินเทอร์เน็ตหรือใช้สิทธิ์ Administrator

## สถานะโปรเจกต์

The project is in early development. The Rust CLI can scan folders and set or randomly select a wallpaper on Windows. The desktop interface and slideshow are planned.

## ฟีเจอร์ที่วางแผนไว้

- จัดการโฟลเดอร์ภาพหลายโฟลเดอร์และค้นหาภาพในโฟลเดอร์ย่อย
- เปลี่ยนวอลเปเปอร์ตามเวลา พร้อมสุ่มภาพแบบไม่ซ้ำจนครบชุด
- ควบคุมโปรแกรมผ่าน System Tray และบันทึกการตั้งค่า
- รองรับหลายจอ แกลเลอรีภาพ ประวัติวอลเปเปอร์ และการตรวจจับไฟล์ที่เปลี่ยนแปลง
- เพิ่มโปรไฟล์วอลเปเปอร์ เครื่องมือปรับภาพ และปุ่มลัดในอนาคต

## เทคโนโลยี

- **แพลตฟอร์มเป้าหมาย:** Windows 11
- **Backend:** Rust
- **Desktop UI ที่วางแผนไว้:** Tauri 2, React และ TypeScript
- **รูปแบบการแจกจ่ายที่วางแผนไว้:** Portable EXE และ Installer

## เริ่มต้นพัฒนา

Install Rust and Cargo, then run the CLI from the repository root:

```powershell
cargo run -p mindswall-cli -- scan "C:\Pictures"
cargo run -p mindswall-cli -- scan --no-recursive "C:\Pictures" "D:\Wallpapers"
cargo run -p mindswall-cli -- set "C:\Pictures\wallpaper.jpg"
cargo run -p mindswall-cli -- random "C:\Pictures"
```

Scanning includes subfolders by default; use `--no-recursive` to restrict it to
the supplied folders or `--recursive` to make the default explicit. Supported
extensions are JPG, JPEG, PNG, and BMP. Errors are reported to stderr. Wallpaper
commands require Windows. The Windows fallback uses the existing fit setting if
the desktop wallpaper API is unavailable.

### Slideshow (Windows)

Copy `config.example.json` to `%APPDATA%\MindsWall\config\config.json` and
edit the folder paths. JSON paths need doubled backslashes. Enable or disable
each folder and set whether it includes subfolders. The interval is in seconds
(60–86400); set `shuffle` to `false` for sequential order. Config changes are
loaded when the slideshow starts.

```powershell
New-Item -ItemType Directory -Force "$env:APPDATA\MindsWall\config"
Copy-Item config.example.json "$env:APPDATA\MindsWall\config\config.json"
# Edit the copied config.json before starting.
cargo run -p mindswall-cli -- run
```

Keep that terminal open. From another terminal, run any of these commands:

```powershell
cargo run -p mindswall-cli -- status
cargo run -p mindswall-cli -- next
cargo run -p mindswall-cli -- previous
cargo run -p mindswall-cli -- pause
cargo run -p mindswall-cli -- resume
cargo run -p mindswall-cli -- config show
```

Press Ctrl+C in the slideshow terminal to stop it. Logs are stored under
`%APPDATA%\MindsWall\data\mindswall.log`. The slideshow and Windows wallpaper
integration still need end-to-end testing in an interactive Windows session.

### Desktop app (development)

Install Node.js, Rust, and the Tauri Windows prerequisites (including WebView2
and the MSVC build tools), then from the project root run:

```powershell
npm.cmd --prefix ui install
npm.cmd --prefix ui run tauri dev
```

Use **Folders** to select image directories, then start the slideshow from
**Overview**. Closing the window hides it to the system tray; use the tray
menu to open or quit the app. **Settings** includes the per-user Windows
startup option. The app and the CLI use the same local configuration and control
pipe. Desktop interactions and startup behavior still require interactive
Windows verification.

The desktop **Gallery** loads images in pages and keeps local thumbnails. You
can set or exclude an image, and restore exclusions in **Settings**. Settings
also contains fit, resolution, orientation, and aspect-ratio filters. **History**
keeps the most recent 100 applied images locally, and **Monitors** lists
connected displays and can apply an image to one display. A running slideshow
can replace a per-monitor selection at its next change.

These utilities remain in development. Folder watching and per-monitor slideshow
sources are not yet implemented. Multi-monitor behavior and the gallery with a
large collection still need interactive Windows verification.

ดูขอบเขตฟีเจอร์ สถาปัตยกรรม และแผนการพัฒนาเพิ่มเติมได้ที่ [Requirement](docs/requirements/requirement.md)

---

© 2026 Namchok Singhachai. MindsWall is released under the [MIT License](LICENSE).
