<form action="{site_url}/index.php?dn=user" method="post">
<div class="form-area">
    <fieldset>
        <legend>{login}:</legend>
        <input class="sinput" name="login" size="35" type="text" maxlength="{maxname}" />
    </fieldset>
    <fieldset>
        <legend>{pass}:</legend>
        <input class="sinput" name="passw" size="35" type="password" maxlength="{maxpass}" />
    </fieldset>
    <br />
    <div class="form-area-apart"> 
        <input name="re" value="login" type="hidden" />
        <input name="to" value="check" type="hidden" />
        <button class="sub" type="submit">{enter}</button>
    </div>
    <div class="form-area-apart"> 
        <p class="user"><a href="index.php?dn=user&amp;re=login&amp;to=lost" title="{send_pass}" rel="nofollow">{send_pass}</a></p>
        <p class="user"><a href="{reglink}" title="{registr}" rel="nofollow">{registr}</a></p>
    </div>
</div>
</form>
