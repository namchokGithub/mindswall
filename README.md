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

ดูขอบเขตฟีเจอร์ สถาปัตยกรรม และแผนการพัฒนาเพิ่มเติมได้ที่ [Requirement](docs/requirements/requirement.md)

---

© 2026 Namchok Singhachai. MindsWall is released under the [MIT License](LICENSE).
