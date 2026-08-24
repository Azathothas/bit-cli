namespace BitTorrent
{
    using System.IO;

    internal class TorrentFile
    {
        private readonly FileInfo fileInfo;

        private readonly long length;

        internal TorrentFile(long len, string path) // : this()
        {
            this.fileInfo = new FileInfo(path);
            this.length = len;
        }

        internal long Length => this.length;

        internal string Path => this.fileInfo.FullName;

        internal string Name => this.fileInfo.Name;
    }
}
