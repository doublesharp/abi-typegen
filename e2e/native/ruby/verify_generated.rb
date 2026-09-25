# frozen_string_literal: true

# Syntax-check and load every generated contract together, catching constant
# collisions that per-file syntax checks cannot detect.
directory = File.expand_path(ARGV.fetch(0))
metadata_only = ARGV.include?("--metadata")
files = Dir.glob(File.join(directory, "*.rb")).sort
raise "No Ruby bindings in #{directory}" if files.empty?

files.each do |file|
  RubyVM::InstructionSequence.compile_file(file)
  source = File.read(file)
  if metadata_only
    raise "Missing ABI constant in #{file}" unless source.match?(/^[A-Z][A-Z0-9_]*_ABI = JSON\.parse/)
    raise "Runtime class in metadata-only #{file}" if source.include?("attr_reader :address, :client, :contract")
    raise "Runtime dependency in metadata-only #{file}" if source.include?('require "eth"')
  end
  require file
end

puts "Verified #{files.length} Ruby bindings#{metadata_only ? ' (metadata only)' : ''}"
