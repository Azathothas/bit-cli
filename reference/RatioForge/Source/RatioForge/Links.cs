namespace RatioForge
{
    internal static class Links
    {
        public const string ProgramPage = "https://github.com/tsautier/RatioForge";

        public const string GitHubPage = "https://github.com/tsautier/RatioForge";

        public const string OriginalAuthorPage = "http://nikolay.it";

        public const string OriginalProjectPage = "https://github.com/NikolayIT/RatioMaster.NET";

        public const string SupportPage = "https://github.com/tsautier/RatioForge";

        public const string AuthorPage = "https://github.com/tsautier/";

        public static void OpenUrl(string url)
        {
            try
            {
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(url) { UseShellExecute = true });
            }
            catch (System.Exception ex)
            {
                System.Windows.Forms.MessageBox.Show("Could not open URL: " + url + "\r\nError: " + ex.Message, "Error", System.Windows.Forms.MessageBoxButtons.OK, System.Windows.Forms.MessageBoxIcon.Error);
            }
        }
    }
}
