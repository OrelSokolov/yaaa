#!/usr/bin/env ruby
# frozen_string_literal: true

# macOS installer for Yaaa.
# Downloads the universal binary from GitHub releases, builds a local .app
# bundle and signs it ad-hoc. Because the file is created on the user's
# machine, Gatekeeper does not treat it as a downloaded/quarantined file.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/OrelSokolov/yaaa/master/install.rb | ruby
#
# Optional environment variables:
#   YAAA_VERSION   - release tag, e.g. "v0.4.3" (default: latest)
#   YAAA_INSTALL   - install directory (default: /Applications)

require 'open-uri'
require 'fileutils'
require 'tmpdir'
require 'rbconfig'

PACKAGE_NAME     = 'yaaa'
APP_NAME         = 'Yaaa'
BUNDLE_ID        = 'com.orelsokolov.yaaa'
GITHUB_USER      = 'OrelSokolov'
GITHUB_REPO      = 'yaaa'
DEFAULT_INSTALL  = '/Applications'
MIN_MACOS_VER    = '10.15'

RAW_BASE_URL = "https://raw.githubusercontent.com/#{GITHUB_USER}/#{GITHUB_REPO}/master"

def error(message)
  warn "Error: #{message}"
  exit 1
end

def macos?
  RbConfig::CONFIG['host_os'].include?('darwin')
end

def version
  ENV.fetch('YAAA_VERSION', 'latest')
end

def install_dir
  ENV.fetch('YAAA_INSTALL', DEFAULT_INSTALL)
end

def binary_url
  "https://github.com/#{GITHUB_USER}/#{GITHUB_REPO}/releases/download/#{version}/#{PACKAGE_NAME}-macos"
end

def icon_url
  "#{RAW_BASE_URL}/assets/logo.icns"
end

def logo_url
  "#{RAW_BASE_URL}/assets/logo.png"
end

def download(url, dest)
  File.open(dest, 'wb') do |file|
    URI.open(url, 'rb') do |stream| # rubocop:disable Security/Open
      while (chunk = stream.read(8192))
        file.write(chunk)
      end
    end
  end
rescue OpenURI::HTTPError => e
  error("failed to download #{url}: #{e.message}")
end

def ensure_executable(path)
  mode = File.stat(path).mode | 0111
  File.chmod(mode, path)
end

def build_app_bundle(binary_path, app_bundle, temp_dir)
  FileUtils.rm_rf(app_bundle)

  macos_dir     = File.join(app_bundle, 'Contents', 'MacOS')
  resources_dir = File.join(app_bundle, 'Contents', 'Resources')

  FileUtils.mkdir_p(macos_dir)
  FileUtils.mkdir_p(resources_dir)

  executable = File.join(macos_dir, PACKAGE_NAME)
  FileUtils.cp(binary_path, executable)
  ensure_executable(executable)

  icon_path = File.join(temp_dir, 'logo.icns')
  if download_optional(icon_url, icon_path)
    FileUtils.cp(icon_path, File.join(resources_dir, 'logo.icns'))
  end

  logo_path = File.join(temp_dir, 'logo.png')
  download_optional(logo_url, logo_path)
  FileUtils.cp(logo_path, File.join(resources_dir, 'logo.png')) if File.exist?(logo_path)

  info_plist = <<~PLIST
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
      <key>CFBundleName</key>
      <string>#{APP_NAME}</string>
      <key>CFBundleDisplayName</key>
      <string>#{APP_NAME}</string>
      <key>CFBundleIdentifier</key>
      <string>#{BUNDLE_ID}</string>
      <key>CFBundleExecutable</key>
      <string>#{PACKAGE_NAME}</string>
      <key>CFBundlePackageType</key>
      <string>APPL</string>
      <key>CFBundleIconFile</key>
      <string>logo.icns</string>
      <key>LSMinimumSystemVersion</key>
      <string>#{MIN_MACOS_VER}</string>
      <key>NSHighResolutionCapable</key>
      <true/>
    </dict>
    </plist>
  PLIST

  File.write(File.join(app_bundle, 'Contents', 'Info.plist'), info_plist)
end

def download_optional(url, dest)
  download(url, dest)
  true
rescue StandardError
  false
end

def codesign(app_bundle)
  system("codesign --force --deep --sign - '#{app_bundle}'") || error('codesign failed')
end

def remove_quarantine(path)
  system("xattr -dr com.apple.quarantine '#{path}' 2>/dev/null")
end

def install
  error('this installer is for macOS only') unless macos?

  puts "Installing #{APP_NAME} (#{version == 'latest' ? 'latest release' : version})..."

  Dir.mktmpdir('yaaa-install-') do |temp_dir|
    binary_path = File.join(temp_dir, PACKAGE_NAME)

    puts "Downloading #{binary_url}..."
    download(binary_url, binary_path)

    app_bundle = File.join(temp_dir, "#{APP_NAME}.app")
    puts 'Building .app bundle...'
    build_app_bundle(binary_path, app_bundle, temp_dir)

    puts 'Signing .app bundle (ad-hoc)...'
    codesign(app_bundle)

    target = File.join(install_dir, "#{APP_NAME}.app")
    puts "Installing to #{target}..."
    FileUtils.rm_rf(target)
    FileUtils.cp_r(app_bundle, target)

    remove_quarantine(target)

    puts "Done! #{APP_NAME} is installed at #{target}"
  end
end

install
