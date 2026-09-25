ExUnit.start()

unless System.get_env("ATG_RPC_URL") do
  ExUnit.configure(exclude: [:anvil])
end
