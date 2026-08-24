namespace RatioForge.Tests
{
    using System;

    using NUnit.Framework;

    using RatioForge;

    [TestFixture]
    public class VersionCheckerTests
    {
        [Test]
        public void CheckNewVersionShouldDetectNewerSemanticVersion()
        {
            var versionChecker = new VersionChecker(string.Empty, () => "1.0.14");

            var hasNewVersion = versionChecker.CheckNewVersion();

            Assert.That(hasNewVersion, Is.True);
        }

        [Test]
        public void CheckNewVersionShouldRejectInvalidRemoteVersion()
        {
            var versionChecker = new VersionChecker(string.Empty, () => "invalid");

            var hasNewVersion = versionChecker.CheckNewVersion();

            Assert.That(hasNewVersion, Is.False);
            Assert.That(versionChecker.RemoteVersion, Is.EqualTo("error"));
        }

        [Test]
        public void PublicVersionShouldComeFromAssemblyMetadata()
        {
            Assert.Multiple((Action)(() =>
            {
                Assert.That(VersionChecker.PublicVersion, Is.EqualTo("1.0.13"));
                Assert.That(VersionChecker.ReleaseDate, Is.EqualTo("11-08-2026"));
            }));
        }
    }
}
