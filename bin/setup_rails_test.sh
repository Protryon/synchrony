#!/bin/bash
rm -rf rails_test
mkdir rails_test
cd rails_test
cat << EOF > ./Gemfile
source 'https://rubygems.org'

ruby '2.5.5'

gem 'rails', '~> 5.2.2', '>= 5.2.2.1'
gem 'sidekiq'
gem 'bootsnap'
gem 'listen'
EOF
bundle install
rails new -s .
mkdir app/workers
cat << EOF > ./app/workers/test_worker.rb
class TestWorker
  include Sidekiq::Worker

  def perform(arg)
    puts "test successful #{arg.to_json}"
  end
end
EOF
# ruby << EOF
# require './config/environment'
# TestWorker.new.perform
# EOF
cd ..
