require 'yaml'

PACKAGE_NAME = 'yaaa'
VERSION = File.read('Cargo.toml').match(/^version\s*=\s*"([^"]+)"/)[1]
ARCH = 'amd64'

namespace :build do
  desc 'Build Debian package'
  task :deb do
  sh 'cargo build --release'

  deb_dir = "yaaa-#{VERSION}"
  dest_dir = "#{deb_dir}/usr/local/bin"

  FileUtils.rm_rf(deb_dir)
  FileUtils.mkdir_p(dest_dir)

  sh "cp target/release/yaaa #{dest_dir}/yaaa"

  control_dir = "#{deb_dir}/DEBIAN"
  FileUtils.mkdir_p(control_dir)

  control_content = <<~CONTROL
    Package: #{PACKAGE_NAME}
    Version: #{VERSION}
    Section: utils
    Priority: optional
    Architecture: #{ARCH}
    Maintainer: YAAA Developer <dev@yaaa.local>
    Description: YAAA Terminal Application
     A modern terminal application built with Rust and egui.
  CONTROL

  File.write("#{control_dir}/control", control_content)

  sh "dpkg-deb --root-owner-group --build #{deb_dir} yaaa_#{VERSION}_#{ARCH}.deb"
  FileUtils.rm_rf(deb_dir)

  puts "Created yaaa_#{VERSION}_#{ARCH}.deb"
  end
end
