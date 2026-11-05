using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Numerics;
using System.Text;
using System.Threading.Tasks;
using Nethereum.JsonRpc.Client;
using NUnit.Framework;

namespace AbiTypegen.Unity.E2E.Tests
{
    public sealed class LocalHttpRpcEditModeTests
    {
        [Test]
        public async Task GenericSimulationAndEstimateUseRealLocalJsonRpcTransport()
        {
            int port;
            using (var reservation = new TcpListener(IPAddress.Loopback, 0))
            {
                reservation.Start();
                port = ((IPEndPoint)reservation.LocalEndpoint).Port;
            }
            var endpoint = new Uri("http://127.0.0.1:" + port + "/");
            using (var server = new HttpListener())
            {
                server.Prefixes.Add(endpoint.AbsoluteUri);
                server.Start();
                var large = (BigInteger.One << 200) + 7;
                var serving = Task.Run(async () =>
                {
                    for (var index = 0; index < 2; index++)
                    {
                        var request = await server.GetContextAsync();
                        string body;
                        using (var reader = new StreamReader(request.Request.InputStream)) body = await reader.ReadToEndAsync();
                        Assert.That(body, Does.Contain(index == 0 ? "eth_call" : "eth_estimateGas"));
                        var result = index == 0 ? "0x1234" : "0x" + large.ToString("x").TrimStart('0');
                        var bytes = Encoding.UTF8.GetBytes("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"" + result + "\"}");
                        request.Response.ContentType = "application/json";
                        request.Response.ContentLength64 = bytes.Length;
                        await request.Response.OutputStream.WriteAsync(bytes, 0, bytes.Length);
                        request.Response.Close();
                    }
                });
                var operations = new AbiTypegenRpcOperations(new RpcClient(endpoint));
                const string address = "0x1111111111111111111111111111111111111111";
                var transaction = AbiTypegenTransaction.Create(address, "0x12", address, value: large);
                Assert.That(await operations.SimulateAsync(transaction, data => data), Is.EqualTo("0x1234"));
                Assert.That(await operations.EstimateGasAsync(transaction), Is.EqualTo(large));
                await serving;
            }
        }
    }
}
