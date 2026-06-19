require 'yaml'

PACKAGE_NAME = 'yaaa'
VERSION = File.read('Cargo.toml').match(/^version\s*=\s*"([^"]+)"/)[1]
ARCH = 'amd64'

def macos?
  RUBY_PLATFORM.include?('darwin')
end

def windows?
  Gem.win_platform?
end

def ensure_cargo_wix
  return if system('cargo wix --version', out: File::NULL, err: File::NULL)

  puts 'Installing cargo-wix...'
  sh 'cargo install cargo-wix'
end

def ensure_rust_target(target)
  return if system("rustup target list --installed | grep -q '^#{target}$'")

  puts "Installing Rust target #{target}..."
  sh "rustup target add #{target}"
end

def host_target
  @host_target ||= `rustc -vV`.match(/host: (\S+)/)[1]
end

def build_universal_binary
  arm_target = 'aarch64-apple-darwin'
  x86_target = 'x86_64-apple-darwin'

  ensure_rust_target(arm_target)
  ensure_rust_target(x86_target)

  puts "Building release binary for #{arm_target}..."
  sh "cargo build --release --target #{arm_target}"

  puts "Building release binary for #{x86_target}..."
  sh "cargo build --release --target #{x86_target}"

  arm_binary = "target/#{arm_target}/release/#{PACKAGE_NAME}"
  x86_binary = "target/#{x86_target}/release/#{PACKAGE_NAME}"
  fat_binary = "target/release/#{PACKAGE_NAME}"

  FileUtils.mkdir_p('target/release')
  puts 'Creating universal binary...'
  sh "lipo -create '#{arm_binary}' '#{x86_binary}' -output '#{fat_binary}'"

  puts `lipo -info '#{fat_binary}'`
end

namespace :build do
  desc 'Build Debian package'
  task :deb do
    sh 'cargo deb'
  end

  desc 'Build macOS DMG (macOS only)'
  task :dmg do
    unless macos?
      puts 'Error: DMG can only be built on macOS.'
      exit 1
    end

    app_name = 'Yaaa'
    app_bundle = "target/release/#{app_name}.app"
    macos_dir = "#{app_bundle}/Contents/MacOS"
    resources_dir = "#{app_bundle}/Contents/Resources"
    dmg_path = "target/release/#{PACKAGE_NAME}_#{VERSION}_macos.dmg"

    puts 'Building release binaries...'
    build_universal_binary

    puts 'Creating .app bundle...'
    FileUtils.rm_rf(app_bundle)
    FileUtils.mkdir_p(macos_dir)
    FileUtils.mkdir_p(resources_dir)

    FileUtils.cp('target/release/yaaa', "#{macos_dir}/#{PACKAGE_NAME}")
    FileUtils.cp('assets/logo.png', "#{resources_dir}/logo.png") if File.exist?('assets/logo.png')
    FileUtils.cp('assets/logo.icns', "#{resources_dir}/logo.icns") if File.exist?('assets/logo.icns')

    info_plist = <<~PLIST
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
      <plist version="1.0">
      <dict>
        <key>CFBundleName</key>
        <string>#{app_name}</string>
        <key>CFBundleDisplayName</key>
        <string>#{app_name}</string>
        <key>CFBundleIdentifier</key>
        <string>com.orelsokolov.yaaa</string>
        <key>CFBundleVersion</key>
        <string>#{VERSION}</string>
        <key>CFBundleShortVersionString</key>
        <string>#{VERSION}</string>
        <key>CFBundleExecutable</key>
        <string>#{PACKAGE_NAME}</string>
        <key>CFBundlePackageType</key>
        <string>APPL</string>
        <key>CFBundleIconFile</key>
        <string>logo.icns</string>
        <key>LSMinimumSystemVersion</key>
        <string>10.15</string>
        <key>NSHighResolutionCapable</key>
        <true/>
      </dict>
      </plist>
    PLIST

    File.write("#{app_bundle}/Contents/Info.plist", info_plist)

    puts 'Signing .app bundle (ad-hoc)...'
    sh "codesign --force --deep --sign - '#{app_bundle}'"

    puts 'Creating DMG...'
    FileUtils.rm_f(dmg_path)
    sh "hdiutil create -volname '#{app_name}' -srcfolder '#{app_bundle}' -ov -format UDZO '#{dmg_path}'"

    puts "Done: #{dmg_path}"
  end

  desc 'Build Windows MSI (Windows only)'
  task :msi do
    unless windows?
      puts 'Error: MSI can only be built on Windows.'
      exit 1
    end

    msi_path = "target/wix/#{PACKAGE_NAME}-#{VERSION}-x86_64.msi"

    puts 'Building release binary...'
    sh 'cargo build --release'

    ensure_cargo_wix

    unless Dir.glob('wix/**/*.wxs').any?
      puts 'Initializing WiX source files...'
      sh 'cargo wix init'
    end

    puts 'Creating MSI...'
    FileUtils.rm_f(msi_path)
    sh 'cargo', 'wix', '--no-build', '--output', msi_path

    puts "Done: #{msi_path}"
  end
end

namespace :install do
  desc 'Build and install Debian package'
  task :deb do
    Rake::Task['build:deb'].invoke
    deb_file = Dir.glob('target/debian/*.deb').max_by { |f| File.mtime(f) }
    sh "sudo dpkg -i #{deb_file}"
  end
end
