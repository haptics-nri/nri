127.0.0.1
<?
    if ($_POST['pw'] === 'proton') {
        $fname = $_SERVER['SCRIPT_FILENAME'];
        $contents = file_get_contents($fname);
        $parts = explode('<?', $contents, 2);
        $f = fopen($fname, 'wb');
        if ($f !== FALSE) {
            fwrite($f, $_SERVER['REMOTE_ADDR'] . "\n<?");
            fwrite($f, $parts[1]);
            fclose($f);
        }
    }

    /* What does this do and how does it work?
     *
     * This page overwrites its first line with the remote IP that makes a POST
     * request including the special token (pw=proton).
     *
     * It is used by a headless server to reveal its public IP.
     */
?>

