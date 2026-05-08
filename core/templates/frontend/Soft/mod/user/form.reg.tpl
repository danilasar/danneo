<script>
$(document).ready(function() {
    $('#refresh').click(function() {
        var t = new Date().getTime();
        $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="" />');
    });
});
</script>
<form action="{site_url}/index.php?dn=user" method="post">
<div class="clear-line"></div>
<div class="form-area">
    <fieldset>
    <legend>{login}</legend>
        <strong>|</strong><input class="width" name="reglogin" size="30" type="text" maxlength="{maxname}" />
    </fieldset>
    <fieldset>
    <legend>{pass}</legend>
        <strong>|</strong><input class="width" name="regpassw" size="30" type="password" maxlength="{maxpass}" />
    </fieldset>
    <fieldset>
    <legend>{re_pass}</legend>
        <strong>|</strong><input class="width" name="regpasswconfir" size="30" type="password" maxlength="{maxpass}" />
    </fieldset>
    <fieldset>
    <legend>{e_mail}</legend>
        <strong>|</strong><input class="width" name="regmail" size="30" type="text" />
    </fieldset>
    <fieldset>
    <legend>{re_e_mail}</legend>
        <strong>|</strong><input class="width" name="regmailconfir" size="30" type="text" />
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
    <legend>Captcha</legend>
    <table width="100%">
    <tbody>
        <tr>
            <td width="100%">
                <strong>|</strong><input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
            </td>
            <td class="ac va pad">
                <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
            </td>
            <td class="ac va pad">
                <button type="button" id="refresh" class="sub">{all_refresh}</button>
            </td>
        </tr>
    </tbody>
    </table>
    </fieldset>
    <!--if-->
    <!--if:control:yes-->
    <fieldset class="standart">
    <legend>{control_word}</legend>
       <p>{control}</p>
       <strong>|</strong><input class="width" id="respon" name="respon" size="30" type="text" />
       <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="re" value="register" type="hidden" />
        <input name="to" value="check" type="hidden" />
        <button type="submit" class="sub">{further}</button>
    </div>
</div>
</form>
