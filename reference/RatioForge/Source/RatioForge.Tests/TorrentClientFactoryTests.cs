namespace RatioForge.Tests
{
    using System;
    using System.Collections.Generic;

    using NUnit.Framework;

    using RatioForge;

    [TestFixture]
    public class TorrentClientFactoryTests
    {
        private static readonly IReadOnlyDictionary<string, string> ExpectedPeerIdPrefixes = new Dictionary<string, string>
        {
            ["BitComet 1.20"] = "-BC0120-",
            ["BitComet 1.03"] = "-BC0103-",
            ["BitComet 0.98"] = "-BC0098-",
            ["BitComet 0.96"] = "-BC0096-",
            ["BitComet 0.93"] = "-BC0093-",
            ["BitComet 0.92"] = "-BC0092-",
            ["Vuze 4.2.0.8"] = "-AZ4208-",
            ["BiglyBT 4.1.0.0"] = "-BI4100-",
            ["Azureus 3.1.1.0"] = "-AZ3110-",
            ["Azureus 3.0.5.0"] = "-AZ3050-",
            ["Azureus 3.0.4.2"] = "-AZ3042-",
            ["Azureus 3.0.3.4"] = "-AZ3034-",
            ["Azureus 3.0.2.2"] = "-AZ3022-",
            ["Azureus 2.5.0.4"] = "-AZ2504-",
            ["uTorrent 3.6.0 (build 46590)"] = "-UT3600-",
            ["uTorrent 3.3.2"] = "-UT3320-",
            ["uTorrent 3.3.0"] = "-UT3300-",
            ["uTorrent 3.2.0"] = "-UT3200-",
            ["uTorrent 2.0.1 (build 19078)"] = "-UT2010-",
            ["uTorrent 1.8.5 (build 17414)"] = "-UT1850-",
            ["uTorrent 1.8.1-beta(11903)"] = "-UT181B-",
            ["uTorrent 1.8.0"] = "-UT1800-",
            ["uTorrent 1.7.7"] = "-UT1770-",
            ["uTorrent 1.7.6"] = "-UT1760-",
            ["uTorrent 1.7.5"] = "-UT1750-",
            ["uTorrent 1.6.1"] = "-UT1610-",
            ["uTorrent 1.6"] = "-UT1600-",
            ["BitTorrent 6.0.3 (8642)"] = "M6-0-3--",
            ["Transmission 2.82 (14160)"] = "-TR2500-",
            ["Transmission 2.92 (14714)"] = "-TR2920-",
            ["Transmission 4.1.3"] = "-TR4130-",
            ["ABC 3.1"] = "A310--",
            ["BitLord 1.1"] = "exbc%01%01LORD",
            ["BTuga 2.1.8"] = "R26---",
            ["BitTornado 0.3.17"] = "T03H-----",
            ["Burst 3.1.0b"] = "Mbrst1-1-3",
            ["BitTyrant 1.1"] = "AZ2500BT",
            ["BitSpirit 3.6.0.200"] = "%2dSP3602",
            ["BitSpirit 3.1.0.077"] = "%00%03BS",
            ["Deluge 2.2.0"] = "-DE220s-",
            ["Deluge 1.3.15"] = "-DE13F0-",
            ["Deluge 1.2.0"] = "-DE1200-",
            ["Deluge 0.5.8.7"] = "-DE0587-",
            ["Deluge 0.5.8.6"] = "-DE0586-",
            ["KTorrent 2.2.1"] = "-KT2210-",
            ["KTorrent 26.04.3"] = "-KT26043-%00",
            ["Gnome BT 0.0.28-1"] = "M3-4-2--",
            ["qBittorrent 5.1.3"] = "-qB5130-",
            ["qBittorrent 5.2.3"] = "-qB5230-",
            ["qBittorrent 5.1.2"] = "-qB5120-",
            ["qBittorrent 4.6.3"] = "-qB4630-",
            ["qBittorrent 4.5.5"] = "-qB4550-",
            ["qBittorrent 4.4.5"] = "-qB4450-",
            ["qBittorrent 4.2.3"] = "-qB4230-",
        };

        public static IEnumerable<string> ClientNames()
        {
            return ExpectedPeerIdPrefixes.Keys;
        }

        [TestCaseSource(nameof(ClientNames))]
        public void GetClientShouldReturnPeerIdForSelectedClient(string clientName)
        {
            TorrentClient client = TorrentClientFactory.GetClient(clientName);

            Assert.Multiple((Action)(() =>
            {
                Assert.That(client.Name, Is.EqualTo(clientName));
                Assert.That(client.PeerID, Does.StartWith(ExpectedPeerIdPrefixes[clientName]));
                Assert.That(DecodeUrlEncodedPeerId(client.PeerID), Has.Length.EqualTo(20));
            }));
        }

        [TestCase("Deluge 2.2.0", "Deluge/2.2.0 libtorrent/2.0.11.0")]
        [TestCase("Deluge 1.3.15", "Deluge 1.3.15")]
        [TestCase("BiglyBT 4.1.0.0", "BiglyBT 4.1.0.0")]
        [TestCase("Transmission 4.1.3", "Transmission/4.1.3")]
        [TestCase("KTorrent 26.04.3", "KTorrent/26.04.3")]
        [TestCase("qBittorrent 5.2.3", "qBittorrent/5.2.3")]
        [TestCase("qBittorrent 5.1.3", "qBittorrent/5.1.3")]
        [TestCase("qBittorrent 5.1.2", "qBittorrent/5.1.2")]
        [TestCase("qBittorrent 4.6.3", "qBittorrent/4.6.3")]
        [TestCase("qBittorrent 4.5.5", "qBittorrent/4.5.5")]
        [TestCase("qBittorrent 4.4.5", "qBittorrent/4.4.5")]
        [TestCase("qBittorrent 4.2.3", "qBittorrent/4.2.3")]
        public void GetClientShouldUseVersionSpecificUserAgent(string clientName, string userAgent)
        {
            TorrentClient client = TorrentClientFactory.GetClient(clientName);

            Assert.That(client.Headers, Does.Contain("User-Agent: " + userAgent));
        }

        private static byte[] DecodeUrlEncodedPeerId(string peerId)
        {
            var bytes = new List<byte>(20);
            for (int i = 0; i < peerId.Length; i++)
            {
                if (peerId[i] == '%' && i + 2 < peerId.Length && IsHex(peerId[i + 1]) && IsHex(peerId[i + 2]))
                {
                    bytes.Add(Convert.ToByte(peerId.Substring(i + 1, 2), 16));
                    i += 2;
                }
                else
                {
                    bytes.Add((byte)peerId[i]);
                }
            }

            return bytes.ToArray();
        }

        private static bool IsHex(char value)
        {
            return (value >= '0' && value <= '9') ||
                (value >= 'A' && value <= 'F') ||
                (value >= 'a' && value <= 'f');
        }
    }
}
