<form action="{site_url}/index.php?dn=user" method="post">
    <fieldset class="userblock">
    <legend>{login}</legend>
        <input name="login" size="22" type="text" maxlength="{maxname}" />
    </fieldset>
    <fieldset class="userblock">
    <legend>{pass}</legend>
        <input name="passw" size="22" type="password" maxlength="{maxpass}" />
    </fieldset>
    <div class="user-link">
        <input name="re" value="login" type="hidden" />
        <input name="to" value="check" type="hidden" />
        <button type="submit" class="sub">{enter}</button>
    </div>
    <div class="user-link">
        <p class="al user"><a href="{site_url}/{linklost}" title="{send_pass}" rel="nofollow">{send_pass}</a></p>
        <p class="al user"><a href="{site_url}/{linkreg}" title="{registr}" rel="nofollow">{registr}</a></p>
    </div>
</form>
