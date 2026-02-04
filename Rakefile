require 'yaml'

PACKAGE_NAME = 'yaaa'
VERSION = File.read('Cargo.toml').match(/^version\s*=\s*"([^"]+)"/)[1]
ARCH = 'amd64'

namespace :build do
  desc 'Build Debian package'
  task :deb do
    sh 'cargo build --release'

    deb_dir = "yaaa-#{VERSION}"
    FileUtils.rm_rf(deb_dir)

    # Bin directory
    bin_dir = "#{deb_dir}/usr/bin"
    FileUtils.mkdir_p(bin_dir)
    sh "cp target/release/yaaa #{bin_dir}/yaaa"

    # Desktop file
    apps_dir = "#{deb_dir}/usr/share/applications"
    FileUtils.mkdir_p(apps_dir)
    sh "cp debian/yaaa.desktop #{apps_dir}/yaaa.desktop"

    # Icons
    icons_base = "#{deb_dir}/usr/share/icons/hicolor"
    [16, 32, 48, 64, 128, 256].each do |size|
      icon_dir = "#{icons_base}/#{size}x#{size}/apps"
      FileUtils.mkdir_p(icon_dir)
      sh "cp debian/icons/#{size}.png #{icon_dir}/yaaa.png"
    end

    # DEBIAN directory
    debian_dir = "#{deb_dir}/DEBIAN"
    FileUtils.mkdir_p(debian_dir)

    control_content = <<~CONTROL
      Package: #{PACKAGE_NAME}
      Version: #{VERSION}
      Section: utils
      Priority: optional
      Architecture: #{ARCH}
      Maintainer: Oleg Orlov <orelcokolov@gmail.com>
      Description: YAAA - Yet Another AI Agent
       A terminal manager to manage console AI Agents by folders.
    CONTROL

    File.write("#{debian_dir}/control", control_content)

    # Postinst script
    sh "cp debian/postinst #{debian_dir}/postinst"

    sh "dpkg-deb --root-owner-group --build #{deb_dir} yaaa_#{VERSION}_#{ARCH}.deb"
    FileUtils.rm_rf(deb_dir)

    puts "Created yaaa_#{VERSION}_#{ARCH}.deb"
  end
end
