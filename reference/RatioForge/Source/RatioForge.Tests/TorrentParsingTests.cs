namespace RatioForge.Tests
{
    using System;
    using System.IO;

    using BitTorrent;

    using NUnit.Framework;

    [TestFixture]
    public class TorrentParsingTests
    {
        [Test]
        public void SingleFileFixtureShouldExposeTorrentMetadata()
        {
            var torrent = new Torrent(FixturePath("single-file.torrent"));

            Assert.Multiple((Action)(() =>
            {
                Assert.That(torrent.SingleFile, Is.True);
                Assert.That(torrent.Name, Is.EqualTo("sample.bin"));
                Assert.That(torrent.Announce, Is.EqualTo("https://tracker.example/announce"));
                Assert.That(torrent.totalLength, Is.EqualTo(12345));
                Assert.That(torrent.Pieces, Is.EqualTo(1));
                Assert.That(torrent.InfoHash, Has.Length.EqualTo(20));
                Assert.That(torrent.PhysicalFiles, Has.Count.EqualTo(1));
                Assert.That(torrent.PhysicalFiles[0].Length, Is.EqualTo(12345));
            }));
        }

        [Test]
        public void MultiFileFixtureShouldAggregateFileMetadata()
        {
            var torrent = new Torrent(FixturePath("multi-file.torrent"));

            Assert.Multiple((Action)(() =>
            {
                Assert.That(torrent.SingleFile, Is.False);
                Assert.That(torrent.Name, Is.EqualTo("folder"));
                Assert.That(torrent.totalLength, Is.EqualTo(300));
                Assert.That(torrent.PhysicalFiles, Has.Count.EqualTo(2));
                Assert.That(torrent.PhysicalFiles[0].Name, Is.EqualTo("part-a.bin"));
                Assert.That(torrent.PhysicalFiles[0].Length, Is.EqualTo(100));
                Assert.That(torrent.PhysicalFiles[1].Name, Is.EqualTo("part-b.bin"));
                Assert.That(torrent.PhysicalFiles[1].Length, Is.EqualTo(200));
            }));
        }

        [Test]
        public void MissingPieceHashesShouldBeRejected()
        {
            Assert.That(
                (Action)(() => new Torrent(FixturePath("missing-pieces.torrent"))),
                Throws.TypeOf<IncompleteTorrentData>().With.Message.EqualTo("No piece hash data"));
        }

        [Test]
        public void MissingAnnounceShouldBeRejected()
        {
            Assert.That(
                (Action)(() => new Torrent(FixturePath("missing-announce.torrent"))),
                Throws.TypeOf<IncompleteTorrentData>().With.Message.EqualTo("No tracker URL"));
        }

        [Test]
        public void DamagedPieceHashesShouldBeRejected()
        {
            Assert.That(
                (Action)(() => new Torrent(FixturePath("damaged-pieces.torrent"))),
                Throws.TypeOf<IncompleteTorrentData>().With.Message.EqualTo("Missing or damaged piece hash codes"));
        }

        [TestCase("truncated-string.torrent")]
        [TestCase("unterminated-integer.torrent")]
        public void TruncatedBencodeShouldBeRejected(string fileName)
        {
            Assert.That(
                (Action)(() => new Torrent(FixturePath(fileName))),
                Throws.TypeOf<TorrentException>().With.Message.Contains("Unexpected end of data"));
        }

        private static string FixturePath(string fileName)
        {
            return Path.Combine(TestContext.CurrentContext.TestDirectory, "Fixtures", fileName);
        }
    }
}
