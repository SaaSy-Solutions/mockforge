using System.Net;
using System.Net.Sockets;
using System.Text;
using MockForge.Sdk;
using Xunit;

namespace MockForge.Sdk.Tests;

/// <summary>
/// Control-plane credential handling (issue #1011). Does not need the CLI.
/// </summary>
public class ManagementTokenTests
{
    [Fact]
    public void ExplicitTokenWinsOverEnv()
    {
        var token = new string('x', 40);
        Assert.Equal(token, MockServer.ResolveManagementToken(token, new string('e', 40)));
    }

    [Fact]
    public void EnvTokenUsedWhenNoOption()
    {
        var env = new string('e', 40);
        Assert.Equal(env, MockServer.ResolveManagementToken(null, env));
    }

    [Fact]
    public void RandomTokenGeneratedPerInstance()
    {
        var a = MockServer.ResolveManagementToken(null, null);
        var b = MockServer.ResolveManagementToken("", "");
        Assert.Matches("^[0-9a-f]{64}$", a);
        Assert.NotEqual(a, b);
    }

    [Fact]
    public async Task VerificationRequestCarriesHeader()
    {
        var probe = new TcpListener(IPAddress.Loopback, 0);
        probe.Start();
        var port = ((IPEndPoint)probe.LocalEndpoint).Port;
        probe.Stop();

        using var listener = new HttpListener();
        listener.Prefixes.Add($"http://127.0.0.1:{port}/");
        listener.Start();

        var token = new string('k', 48);
        string? seenPath = null;
        string? seenToken = null;
        var serve = Task.Run(async () =>
        {
            var ctx = await listener.GetContextAsync();
            seenPath = ctx.Request.Url?.AbsolutePath;
            seenToken = ctx.Request.Headers[MockServer.ManagementTokenHeader];
            var body = Encoding.UTF8.GetBytes("{\"matched\":true,\"count\":0,\"matches\":[]}");
            ctx.Response.ContentType = "application/json";
            ctx.Response.ContentLength64 = body.Length;
            await ctx.Response.OutputStream.WriteAsync(body);
            ctx.Response.Close();
        });

        using var server = new MockServer(new MockServerConfig
        {
            Port = port,
            ManagementToken = token
        });
        Assert.Equal(token, server.ManagementToken);
        await server.VerifyAsync(new VerificationRequest(), VerificationCount.Never());
        await serve;

        Assert.Equal("/api/verification/verify", seenPath);
        Assert.Equal(token, seenToken);
    }
}
