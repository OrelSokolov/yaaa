require 'yaml'

PACKAGE_NAME = 'yaaa'
VERSION = File.read('Cargo.toml').match(/^version\s*=\s*"([^"]+)"/)[1]
ARCH = 'amd64'

namespace :build do
  desc 'Build Debian package'
  task :deb do
    sh 'cargo deb'
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
